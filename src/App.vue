<script setup lang="ts">
import { usePreferredDark } from '@vueuse/core'
import { watchEffect } from 'vue'
import AppLayout from '@/components/AppLayout.vue'

const isDark = usePreferredDark()

watchEffect(() => {
  const { classList } = document.documentElement
  if (isDark.value)
    classList.add('dark')
  else
    classList.remove('dark')
})
</script>

<template>
  <AppLayout />
</template>

<style>
html {
  color-scheme: light dark;
}

#app {
  height: 100vh;
  overflow: hidden;
}
</style>
