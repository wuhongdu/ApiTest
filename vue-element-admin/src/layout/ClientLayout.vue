<template>
  <div class="client-layout">
    <router-view />
  </div>
</template>

<script>
export default {
  name: 'ClientLayout'
}
</script>

<style lang="scss" scoped>
.client-layout {
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #1c1f26;
}
</style>
