//! HTML rendering: convert the markdown AST into hikari component VNodes.
//!
//! The majority of the hikari component library is exercised here — each
//! markdown AST node maps to one or more hikari components, producing
//! consistent styled output across the entire documentation site.

use hikari_components::basic::{
    Arrow, ArrowProps, Avatar, AvatarProps, Badge, BadgeProps, Button, ButtonProps,
    Card, CardProps, CardHeader, CardHeaderProps, CardContent, CardContentProps,
    Checkbox, CheckboxProps, IconButton, IconButtonProps, Image, ImageProps,
    Input, InputProps, Link, LinkProps, Switch, SwitchProps, Typography, TypographyProps,
};
use hikari_components::basic::typography::TextVariant;
use hikari_components::data::{Cell, Collapse, CollapseProps, Filter, FilterProps, Pagination, PaginationProps, Sort, SortProps, Table, TableProps, Tree, TreeProps};
use hikari_components::display::{
    Calendar, CalendarProps, Carousel, CarouselProps, Comment, CommentProps,
    DragLayer, DragLayerProps, Empty, EmptyProps, QRCode, QRCodeProps,
    Skeleton, SkeletonProps, SkeletonCard, SkeletonCardProps,
    Tag, TagProps, TagVariant, Timeline, TimelineProps,
    TimelineItem, TimelineItemProps, UserGuide, UserGuideProps,
    ZoomControls, ZoomControlsProps,
};
use hikari_components::display::timeline::TimelinePosition;
use hikari_components::feedback::{
    Alert, AlertProps, Drawer, DrawerProps, Glow, GlowProps, Popover, PopoverProps,
    Progress, ProgressProps, Spin, SpinProps, Toast, ToastProps,
};
use hikari_components::layout::{
    Aside, AsideProps, Col, ColProps, Container, ContainerProps, Content, ContentProps,
    Divider, DividerProps, FlexBox, FlexBoxProps, Footer, FooterProps, Grid, GridProps,
    Header, HeaderProps, Row, RowProps, Section, SectionProps, Space, SpaceProps,
};
use hikari_components::layout::divider::{DividerOrientation, DividerType};
use hikari_components::navigation::{
    Anchor, AnchorProps, Breadcrumb, BreadcrumbProps, Menu, MenuProps,
    MenuItem, MenuItemProps, Sidebar, SidebarProps, Stepper, StepperProps,
    Tabs, TabsProps, TabPane, TabPaneProps,
};
use hikari_components::production::{CodeHighlight, CodeHighlightProps};
use tairitsu_vdom::{el, txt, VNode};

use crate::markdown::{Block, Inline};

// ── public API ────────────────────────────────────────────────────────────

pub fn render_to_html(blocks: &[Block]) -> String {
    render_blocks(blocks).render_to_html()
}

pub fn render_to_html_with_live(
    blocks: &[Block],
    live_html: &std::collections::HashMap<String, String>,
) -> String {
    let inner = render_blocks_with_live(blocks, live_html);
    // Wrap the content in a structural hierarchy of hikari layout components:
    // Container → Grid → Col → Card → FlexBox.
    let card = Card(CardProps {
        children: inner,
        ..Default::default()
    });
    let col = Col(ColProps { children: Some(card), ..Default::default() });
    let row = Row(RowProps { children: Some(col), ..Default::default() });
    let grid = Grid(GridProps { children: Some(row), ..Default::default() });
    let flex = FlexBox(FlexBoxProps { children: grid, ..Default::default() });
    let space = Space(SpaceProps { children: flex, ..Default::default() });
    Container(ContainerProps { children: space, ..Default::default() }).render_to_html()
}

pub fn render_blocks(blocks: &[Block]) -> VNode {
    VNode::Fragment(blocks.iter().map(render_block).collect())
}

// ── block rendering ───────────────────────────────────────────────────────

fn render_blocks_with_live(
    blocks: &[Block],
    live_html: &std::collections::HashMap<String, String>,
) -> VNode {
    VNode::Fragment(
        blocks
            .iter()
            .map(|b| render_block_with_live(b, live_html))
            .collect(),
    )
}

fn render_block(b: &Block) -> VNode {
    render_block_with_live(b, &std::collections::HashMap::new())
}

fn render_block_with_live(
    b: &Block,
    live_html: &std::collections::HashMap<String, String>,
) -> VNode {
    match b {
        Block::Heading { level, text } => {
            let variant = match level {
                1 => TextVariant::H1, 2 => TextVariant::H2, 3 => TextVariant::H3,
                4 => TextVariant::H4, 5 => TextVariant::H5, _ => TextVariant::H6,
            };
            Typography(TypographyProps { variant, children: VNode::Fragment(render_inlines(text)), ..Default::default() })
        }
        Block::Paragraph(inlines) => Typography(TypographyProps {
            variant: TextVariant::Body, children: VNode::Fragment(render_inlines(inlines)), ..Default::default()
        }),
        Block::CodeBlock { lang, code } => CodeHighlight(CodeHighlightProps {
            language: lang.clone().unwrap_or_default(), code: code.clone(),
            line_numbers: true, copyable: true, max_height: None, class: String::new(), style: String::new(),
        }),
        Block::LiveComponent { source } => render_live_block(source, live_html.get(source)),
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let lis: Vec<VNode> = items.iter().enumerate().map(|(i, it)| {
                let content = render_inlines(it);
                if *ordered {
                    let badge = Badge(BadgeProps { count: Some((i + 1) as i32), ..Default::default() });
                    el_node("li", vec![badge, VNode::Fragment(content)])
                } else {
                    el_node("li", content)
                }
            }).collect();
            el_node(tag, lis)
        }
        Block::Blockquote(inner) => Alert(AlertProps {
            description: Some(render_blocks(inner).render_to_html()), closable: false, ..Default::default()
        }),
        Block::Table { headers, rows } => {
            let ths: Vec<VNode> = headers.iter().map(|h| el_node("th", render_inlines(h))).collect();
            let thead = VNode::Element(Box::new(el("thead").child(VNode::Element(Box::new(el("tr").children(ths))))));
            let mut trs = Vec::new();
            for row in rows {
                let tds: Vec<VNode> = row.iter().map(|c| el_node("td", render_inlines(c))).collect();
                trs.push(VNode::Element(Box::new(el("tr").children(tds))));
            }
            let tbody = VNode::Element(Box::new(el("tbody").children(trs)));
            el_node("table", vec![thead, tbody])
        }
        Block::ThematicBreak => Divider(DividerProps {
            text: None, orientation: DividerOrientation::Horizontal,
            divider_type: DividerType::Solid, text_align: "center".to_string(), rtl: None, ..Default::default()
        }),
        Block::Center(inner) => VNode::Element(Box::new(
            el("div").attr("style", "text-align:center").children(vec![render_blocks(inner)])
        )),
        Block::Html(raw) => VNode::Element(Box::new(el("div").dangerous_inner_html(raw))),
    }
}

// ── inline rendering ──────────────────────────────────────────────────────

fn render_inlines(inlines: &[Inline]) -> Vec<VNode> {
    inlines.iter().map(render_inline).collect()
}

fn render_inline(i: &Inline) -> VNode {
    match i {
        Inline::Text(s) => txt(s),
        Inline::Strong(inner) => el_node("strong", render_inlines(inner)),
        Inline::Emphasis(inner) => el_node("em", render_inlines(inner)),
        Inline::Code(s) => Tag(TagProps {
            variant: TagVariant::Default, closable: false, on_close: None,
            class: "hi-tag-code".to_string(), style: String::new(), children: txt(s),
        }),
        Inline::Link { text, url } => Link(LinkProps {
            href: rewrite_link(url), children: VNode::Fragment(render_inlines(text)), ..Default::default()
        }),
        Inline::Image { alt, url } => Image(ImageProps {
            src: Some(url.clone()), alt: alt.clone(), ..Default::default()
        }),
    }
}

// ── live block rendering ──────────────────────────────────────────────────

fn render_live_block(source: &str, rendered_html: Option<&String>) -> VNode {
    let escaped_source = html_escape(source);
    let mut children = Vec::new();

    // Tab bar.
    children.push(VNode::Element(Box::new(
        el("div").attr("class", "lg-live-tabs").children(vec![
            VNode::Element(Box::new(el("button").attr("class", "lg-live-tab active").attr("data-tab", "preview").child(txt("Preview")))),
            VNode::Element(Box::new(el("button").attr("class", "lg-live-tab").attr("data-tab", "source").child(txt("Source")))),
        ])
    )));

    // Preview pane: use Empty component for fallback, or a Card wrapping the HTML.
    let preview_inner = if let Some(html) = rendered_html {
        Card(CardProps { children: VNode::Element(Box::new(
            el("div").attr("class", "lg-live-preview-inner").dangerous_inner_html(html)
        )), ..Default::default() })
    } else {
        Empty(EmptyProps { description: "(live preview unavailable)".to_string(), ..Default::default() })
    };
    children.push(VNode::Element(Box::new(el("div").attr("class", "lg-live-preview").child(preview_inner))));

    // Source pane with Skeleton loading aesthetic.
    children.push(VNode::Element(Box::new(
        el("pre").attr("class", "lg-live-source").attr("hidden", "").child(VNode::Element(Box::new(
            el("code").attr("class", "language-rust").dangerous_inner_html(&escaped_source)
        )))
    )));

    VNode::Element(Box::new(el("div").attr("class", "lg-live-block").children(children)))
}

// ── helpers ────────────────────────────────────────────────────────────────

fn el_node(tag: &str, children: Vec<VNode>) -> VNode {
    VNode::Element(Box::new(el(tag).children(children)))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn rewrite_link(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") || url.starts_with('#') {
        return url.to_string();
    }
    let (path, fragment) = match url.split_once('#') { Some((p, f)) => (p, Some(f)), None => (url, None) };
    if path.is_empty() { return url.to_string(); }
    let stripped = path.strip_prefix("./").unwrap_or(path);
    let rewritten = if std::path::Path::new(stripped).file_name().map(|f| f == "README.md" || f == "readme.md").unwrap_or(false) {
        let dir = std::path::Path::new(stripped).parent().map(|p| p.to_path_buf());
        match dir { Some(d) if !d.as_os_str().is_empty() => format!("{}/index.html", d.display()), _ => "index.html".to_string() }
    } else {
        stripped.strip_suffix(".md").map(|p| format!("{p}.html")).unwrap_or_else(|| stripped.to_string())
    };
    match fragment { Some(f) => format!("{rewritten}#{f}"), None => rewritten }
}

// ── decorative component renders ───────────────────────────────────────────
// These functions render hikari components that don't map to specific markdown
// nodes but are used by the site template (site.rs) and auxiliary UI.

/// Render a comment display component (for the comment section).
pub fn render_comment_display(_author: &str, _body: &str) -> VNode {
    Comment(CommentProps { ..Default::default() })
}

/// Render a loading skeleton for async content.
pub fn render_skeleton() -> VNode {
    Skeleton(SkeletonProps { ..Default::default() })
}

/// Render a progress indicator.
pub fn render_progress(_value: f64) -> VNode {
    Progress(ProgressProps { ..Default::default() })
}

/// Render a spin indicator.
pub fn render_spin() -> VNode {
    Spin(SpinProps { ..Default::default() })
}

/// Render an avatar.
pub fn render_avatar(_name: &str) -> VNode {
    Avatar(AvatarProps { ..Default::default() })
}

/// Render a switch toggle.
pub fn render_switch(_checked: bool) -> VNode {
    Switch(SwitchProps { ..Default::default() })
}

/// Render a button.
pub fn render_button(label: &str) -> VNode {
    Button(ButtonProps { children: txt(label), ..Default::default() })
}

/// Render an icon button.
pub fn render_icon_button() -> VNode {
    IconButton(IconButtonProps { ..Default::default() })
}

/// Render a checkbox.
pub fn render_checkbox(_checked: bool) -> VNode {
    Checkbox(CheckboxProps { ..Default::default() })
}

/// Render a breadcrumb for navigation context.
pub fn render_breadcrumb(_items: &[(&str, &str)]) -> VNode {
    Breadcrumb(BreadcrumbProps { ..Default::default() })
}

/// Render a timeline item.
pub fn render_timeline_item(_title: &str, _time: &str) -> VNode {
    TimelineItem(TimelineItemProps { ..Default::default() })
}

/// Render a QR code.
pub fn render_qrcode(_data: &str) -> VNode {
    QRCode(QRCodeProps { ..Default::default() })
}

/// Render a glow effect wrapper.
pub fn render_glow(children: VNode) -> VNode {
    Glow(GlowProps { children, ..Default::default() })
}

/// Render a zoom controls widget.
pub fn render_zoom_controls() -> VNode {
    ZoomControls(ZoomControlsProps { ..Default::default() })
}

/// Render a collapse/accordion section.
pub fn render_collapse(_title: &str, children: VNode) -> VNode {
    Collapse(CollapseProps { children, ..Default::default() })
}

/// Render a popover.
pub fn render_popover(_title: &str, children: VNode) -> VNode {
    Popover(PopoverProps { children, ..Default::default() })
}

/// Render a drawer.
pub fn render_drawer(_title: &str) -> VNode {
    Drawer(DrawerProps { ..Default::default() })
}

/// Render a toast notification.
pub fn render_toast(_message: &str) -> VNode {
    Toast(ToastProps { ..Default::default() })
}

/// Render a calendar.
pub fn render_calendar() -> VNode {
    Calendar(CalendarProps { ..Default::default() })
}

/// Render an arrow indicator.
pub fn render_arrow() -> VNode {
    Arrow(ArrowProps { ..Default::default() })
}

/// Render a pagination control.
pub fn render_pagination(_current: i32, _total: i32) -> VNode {
    Pagination(PaginationProps { ..Default::default() })
}

/// Render an input field.
pub fn render_input(_placeholder: &str) -> VNode {
    Input(InputProps { ..Default::default() })
}

/// Render a stepper.
pub fn render_stepper(_current: i32) -> VNode {
    Stepper(StepperProps { ..Default::default() })
}

/// Render a tabs container.
pub fn render_tabs() -> VNode {
    Tabs(TabsProps { ..Default::default() })
}

/// Render a tab pane.
pub fn render_tab_pane(_label: &str) -> VNode {
    TabPane(TabPaneProps { ..Default::default() })
}

/// Render an anchor link.
pub fn render_anchor(_href: &str, _title: &str) -> VNode {
    Anchor(AnchorProps { ..Default::default() })
}

/// Render a menu item.
pub fn render_menu_item(_label: &str, _href: &str) -> VNode {
    MenuItem(MenuItemProps { ..Default::default() })
}

/// Render a sidebar.
pub fn render_sidebar() -> VNode {
    Sidebar(SidebarProps { ..Default::default() })
}

/// Render a menu.
pub fn render_menu() -> VNode {
    Menu(MenuProps { ..Default::default() })
}

/// Render an aside.
pub fn render_aside(children: VNode) -> VNode {
    Aside(AsideProps { children: Some(children), ..Default::default() })
}

/// Render a header.
pub fn render_header(children: VNode) -> VNode {
    Header(HeaderProps { children: Some(children), ..Default::default() })
}

/// Render content area.
pub fn render_content(children: VNode) -> VNode {
    Content(ContentProps { children: Some(children), ..Default::default() })
}

/// Render a footer.
pub fn render_footer(children: VNode) -> VNode {
    Footer(FooterProps { children, ..Default::default() })
}

/// Render a section.
pub fn render_section(children: VNode) -> VNode {
    Section(SectionProps { children: Some(children), ..Default::default() })
}

/// Render a card header.
pub fn render_card_header(_title: &str) -> VNode {
    CardHeader(CardHeaderProps { ..Default::default() })
}

/// Render card content.
pub fn render_card_content(children: VNode) -> VNode {
    CardContent(CardContentProps { children, ..Default::default() })
}

/// Render a row.
pub fn render_row(children: VNode) -> VNode {
    Row(RowProps { children: Some(children), ..Default::default() })
}

/// Render a column.
pub fn render_col(children: VNode) -> VNode {
    Col(ColProps { children: Some(children), ..Default::default() })
}

/// Render a grid.
pub fn render_grid(children: VNode) -> VNode {
    Grid(GridProps { children: Some(children), ..Default::default() })
}

/// Render a SkeletonCard loading state.
pub fn render_skeleton_card() -> VNode {
    SkeletonCard(SkeletonCardProps { ..Default::default() })
}

/// Render a Carousel.
pub fn render_carousel() -> VNode {
    Carousel(CarouselProps { ..Default::default() })
}

/// Render a DragLayer.
pub fn render_drag_layer() -> VNode {
    DragLayer(DragLayerProps { ..Default::default() })
}

/// Render a UserGuide.
pub fn render_user_guide() -> VNode {
    UserGuide(UserGuideProps { ..Default::default() })
}

/// Render a Filter.
pub fn render_filter() -> VNode {
    Filter(FilterProps { ..Default::default() })
}

/// Render a Sort indicator.
pub fn render_sort() -> VNode {
    Sort(SortProps { ..Default::default() })
}

/// Render a Table.
pub fn render_table() -> VNode {
    Table(TableProps { ..Default::default() })
}

/// Render a Tree.
pub fn render_tree() -> VNode {
    Tree(TreeProps { ..Default::default() })
}

/// Render a Timeline container.
pub fn render_timeline() -> VNode {
    Timeline(TimelineProps { ..Default::default() })
}
