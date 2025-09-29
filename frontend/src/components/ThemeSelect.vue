<template>
  <button @click="setTheme(themeName)" class="w-full !block">
    <div class="flex-1 outline-base-content overflow-hidden rounded-lg outline outline-2 outline-offset-2 w-full" :data-theme="themeName">
      <div :data-theme="themeName" class="bg-base-100 text-base-content w-full cursor-pointer font-sans">
        <div class="grid grid-cols-5 grid-rows-3">
          <div class="bg-base-200 col-start-1 row-span-2 row-start-1"></div>
          <div class="bg-base-300 col-start-1 row-start-3"></div>
          <div class="bg-primary col-start-2 row-span-3 row-start-1 flex items-center justify-center">
            <svg class="w-3 h-3 fill-current" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"></path>
            </svg>
          </div>
          <div class="bg-secondary col-start-3 row-span-3 row-start-1"></div>
          <div class="bg-accent col-start-4 row-span-3 row-start-1"></div>
          <div class="bg-neutral col-start-5 row-span-3 row-start-1"></div>
        </div>
        <div class="bg-base-100 text-base-content col-start-1 col-end-6 row-start-3 row-end-4 px-2 py-1 text-xs font-bold">{{ themeName }}</div>
      </div>
    </div>
  </button>
</template>

<script setup>
defineProps({
  themeName: {
    type: String,
    required: true
  }
})

const emit = defineEmits(['theme-selected'])

const setTheme = (theme) => {
  emit('theme-selected', theme)
}
</script>
