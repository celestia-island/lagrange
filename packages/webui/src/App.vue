<script setup lang="ts">
const ready = true;
</script>

<template>
  <div class="lagrange-shell">
    <nav class="lagrange-sidebar">
      <div class="lagrange-nav-label">Lagrange webui</div>
      <div class="lagrange-nav-item" :data-ready="ready">Pipeline ready</div>
    </nav>
    <main class="lagrange-content">
      <p>Webui OK — hikari integration is the next slice.</p>
    </main>
  </div>
</template>

<style>
.lagrange-shell { display: flex; min-height: 100vh; }
.lagrange-sidebar { width: 240px; border-right: 1px solid #e0e0e0; padding: 1rem; }
.lagrange-nav-label { font-weight: 600; }
.lagrange-nav-item { padding: 0.25rem 0.5rem; }
.lagrange-content { flex: 1; padding: 2rem; }
</style>
