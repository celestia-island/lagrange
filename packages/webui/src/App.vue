<script setup lang="ts">
// P3 slice 1: prove the Vue + hikari pipeline works.
// The knowledge.json integration (sidebar tree from documents/chunks) is the
// next slice — this scaffold proves the build toolchain compiles and mounts.
const ready = true;
</script>

<template>
  <div class="lagrange-shell">
    <nav class="lagrange-sidebar">
      <div class="lagrange-nav-label">Lagrange webui scaffold</div>
      <div class="lagrange-nav-item" :data-ready="ready">Pipeline ready</div>
    </nav>
    <main class="lagrange-content">
      <p>Webui scaffold OK.</p>
    </main>
  </div>
</template>

<style>
.lagrange-shell { display: flex; min-height: 100vh; }
.lagrange-sidebar { width: 240px; border-right: 1px solid #e0e0e0; padding: 1rem; }
.lagrange-nav-label { font-weight: 600; margin-bottom: 0.25rem; }
.lagrange-nav-item { padding: 0.25rem 0.5rem; }
.lagrange-content { flex: 1; padding: 2rem; }
</style>
