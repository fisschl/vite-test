<script setup lang="ts">
import { Hash } from 'lucide-vue-next'

const route = useRoute()

const examples = computed(() => {
  return [
    {
      name: '计算文件哈希值',
      path: '/file-hash',
      icon: markRaw(Hash),
      isActive: route.path.startsWith('/file-hash'),
    },
  ]
})

const activeExample = computed(() => {
  return examples.value.find(example => example.isActive)
})
</script>

<template>
  <div class="flex h-screen min-h-0">
    <ElMenu
      :default-active="activeExample?.path"
      class="h-full"
      router
      :class="$style.aside"
    >
      <ElMenuItem
        v-for="example in examples"
        :key="example.name"
        :index="example.path"
      >
        <ElIcon>
          <component :is="example.icon" />
        </ElIcon>
        <span>{{ example.name }}</span>
      </ElMenuItem>
    </ElMenu>
    <main
      class="flex-1 overflow-hidden relative transition-colors duration-200"
    >
      <RouterView />
    </main>
  </div>
</template>

<style module>
.aside {
  width: 220px;
}
</style>
