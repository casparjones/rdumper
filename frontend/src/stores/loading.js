import { ref, reactive } from 'vue'

// Global loading state
const isLoading = ref(false)
const loadingPages = reactive(new Set())

export function useLoading() {
  const startLoading = (pageName) => {
    loadingPages.add(pageName)
    isLoading.value = true
  }

  const stopLoading = (pageName) => {
    loadingPages.delete(pageName)
    if (loadingPages.size === 0) {
      isLoading.value = false
    }
  }

  const isPageLoading = (pageName) => {
    return loadingPages.has(pageName)
  }

  const clearAllLoading = () => {
    loadingPages.clear()
    isLoading.value = false
  }

  return {
    isLoading,
    startLoading,
    stopLoading,
    isPageLoading,
    clearAllLoading
  }
}
