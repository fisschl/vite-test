<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { File, Key } from 'lucide-vue-next'
import { ref } from 'vue'

const filePath = ref('')
const hashResult = ref('')

async function selectFile() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{
        name: 'All Files',
        extensions: ['*'],
      }],
    })
    if (!selected)
      return
    filePath.value = selected
    await calculateHash()
  }
  catch (error) {

  }
}

async function calculateHash() {
  if (!filePath.value)
    return

  hashResult.value = ''

  try {
    const result = await invoke('calculate_file_hash', { filePath: filePath.value })
    hashResult.value = result as string
  }
  catch (error) {
    ElNotification({
      title: '计算哈希值失败',
      message: String(error),
      type: 'error',
    })
  }
}
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto">
    <ElCard class="border border-gray-200 dark:border-gray-600">
      <ElForm>
        <ElFormItem label="选择文件">
          <div class="flex gap-2">
            <ElInput
              v-model="filePath"
              readonly
              placeholder="点击选择文件..."
              class="flex-1"
            >
              <template #prefix>
                <ElIcon>
                  <File />
                </ElIcon>
              </template>
            </ElInput>
            <ElButton type="primary" @click="selectFile">
              选择文件
            </ElButton>
          </div>
        </ElFormItem>
        <ElFormItem v-if="hashResult" label="哈希结果">
          <ElInput
            :value="hashResult"
            readonly
            class="font-mono text-sm"
          >
            <template #prefix>
              <ElIcon>
                <Key />
              </ElIcon>
            </template>
          </ElInput>
        </ElFormItem>
      </ElForm>
    </ElCard>
  </div>
</template>
