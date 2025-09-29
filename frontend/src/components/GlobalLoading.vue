<template>
  <div 
    v-show="isLoading" 
    class="fixed top-0 left-0 right-0 z-50 h-1 bg-base-300/20 overflow-hidden"
    :class="{ 'opacity-0': !isLoading, 'opacity-100': isLoading }"
    style="transition: opacity 0.4s ease-in-out"
  >
    <div 
      class="h-full relative"
      :style="{ 
        width: '15%',
        transform: `translateX(${position}px)`,
        transition: 'transform 0.05s linear'
      }"
    >
      <!-- Knight Rider light with glow effect -->
      <div class="absolute inset-0 bg-gradient-to-r from-transparent via-primary/80 to-transparent rounded-full"></div>
      <div class="absolute inset-0 bg-gradient-to-r from-transparent via-primary to-transparent rounded-full blur-sm"></div>
      <div class="absolute inset-0 bg-gradient-to-r from-transparent via-primary/60 to-transparent rounded-full blur-md"></div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, onMounted, onUnmounted } from 'vue'

const props = defineProps({
  isLoading: {
    type: Boolean,
    default: false
  }
})

const position = ref(0)
let animationId = null
let direction = 1
let speed = 3
let maxPosition = 0

const updateMaxPosition = () => {
  maxPosition = window.innerWidth - (window.innerWidth * 0.15)
}

const animate = () => {
  position.value += speed * direction
  
  // Reverse direction at edges with smooth bounce
  if (position.value >= maxPosition) {
    direction = -1
    position.value = maxPosition
  } else if (position.value <= 0) {
    direction = 1
    position.value = 0
  }
  
  if (props.isLoading) {
    animationId = requestAnimationFrame(animate)
  }
}

watch(() => props.isLoading, (newValue) => {
  if (newValue) {
    // Start Knight Rider animation
    updateMaxPosition()
    position.value = 0
    direction = 1
    speed = 3
    animate()
  } else {
    // Stop animation
    if (animationId) {
      cancelAnimationFrame(animationId)
      animationId = null
    }
  }
})

onMounted(() => {
  updateMaxPosition()
  window.addEventListener('resize', updateMaxPosition)
})

onUnmounted(() => {
  if (animationId) {
    cancelAnimationFrame(animationId)
  }
  window.removeEventListener('resize', updateMaxPosition)
})
</script>
