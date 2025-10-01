// API Base URL - automatische Domain-Erkennung mit Environment-basiertem Port
const getApiBaseUrl = () => {
  // Wenn VITE_API_URL explizit gesetzt ist, verwende diese
  if (import.meta.env.VITE_API_URL) {
    return import.meta.env.VITE_API_URL
  }
  
  // Automatische Domain-Erkennung
  const protocol = window.location.protocol
  const hostname = window.location.hostname
  const port = import.meta.env.VITE_API_PORT || '443'
  
  // Für Development: localhost mit konfigurierbarem Port
  if (hostname === 'localhost' || hostname === '127.0.0.1') {
    return `${protocol}//${hostname}:${port}`
  }
  
  // Für Production: gleiche Domain mit konfigurierbarem Port
  return `${protocol}//${hostname}:${port}`
}

const API_BASE_URL = getApiBaseUrl()

// Generic API client
class ApiClient {
  constructor(baseUrl = API_BASE_URL) {
    this.baseUrl = baseUrl
  }

  async request(endpoint, options = {}) {
    const url = `${this.baseUrl}${endpoint}`
    
    const config = {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    }

    if (config.body && typeof config.body === 'object') {
      config.body = JSON.stringify(config.body)
    }

    try {
      const response = await fetch(url, config)
      
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
        throw new Error(errorData.error || `HTTP ${response.status}`)
      }

      return await response.json()
    } catch (error) {
      console.error(`API Error (${endpoint}):`, error)
      throw error
    }
  }

  // GET request
  async get(endpoint, params = {}) {
    const searchParams = new URLSearchParams(params)
    const url = Object.keys(params).length > 0 ? `${endpoint}?${searchParams}` : endpoint
    return this.request(url, { method: 'GET' })
  }

  // POST request
  async post(endpoint, data = {}) {
    return this.request(endpoint, {
      method: 'POST',
      body: data,
    })
  }

  // PUT request
  async put(endpoint, data = {}) {
    return this.request(endpoint, {
      method: 'PUT',
      body: data,
    })
  }

  // DELETE request
  async delete(endpoint) {
    return this.request(endpoint, { method: 'DELETE' })
  }
}

// Create API client instance
const apiClient = new ApiClient()

// Database Configs API
export const databaseConfigsApi = {
  // List all database configurations
  list(params = {}) {
    return apiClient.get('/api/database-configs', params)
  },

  // Get a specific database configuration
  get(id) {
    return apiClient.get(`/api/database-configs/${id}`)
  },

  // Create a new database configuration
  create(data) {
    return apiClient.post('/api/database-configs', data)
  },

  // Update a database configuration
  update(id, data) {
    return apiClient.put(`/api/database-configs/${id}`, data)
  },

  // Delete a database configuration
  delete(id) {
    return apiClient.delete(`/api/database-configs/${id}`)
  },

  // Test database connection
  test(id) {
    return apiClient.post(`/api/database-configs/${id}/test`)
  },

  // Check database permissions
  checkPermissions(id) {
    return apiClient.get(`/api/database-configs/${id}/permissions`)
  },

  // Get available databases for a connection
  getDatabases(id) {
    return apiClient.get(`/api/database-configs/${id}/databases`)
  }
}

// Tasks API
export const tasksApi = {
  list(params = {}) {
    return apiClient.get('/api/tasks', params)
  },

  get(id) {
    return apiClient.get(`/api/tasks/${id}`)
  },

  create(data) {
    return apiClient.post('/api/tasks', data)
  },

  update(id, data) {
    return apiClient.put(`/api/tasks/${id}`, data)
  },

  delete(id) {
    return apiClient.delete(`/api/tasks/${id}`)
  },

  run(id) {
    return apiClient.post(`/api/tasks/${id}/run`)
  },

  toggle(id) {
    return apiClient.post(`/api/tasks/${id}/toggle`)
  }
}

// Jobs API
export const jobsApi = {
  list(params = {}) {
    return apiClient.get('/api/jobs', params)
  },

  get(id) {
    return apiClient.get(`/api/jobs/${id}`)
  },

  cancel(id) {
    return apiClient.post(`/api/jobs/${id}/cancel`)
  },

  delete(id) {
    return apiClient.delete(`/api/jobs/${id}`)
  },

  logs(id) {
    return apiClient.get(`/api/jobs/${id}/logs`)
  },

  active() {
    return apiClient.get('/api/jobs/active')
  },

  detailedProgress(id) {
    return apiClient.get(`/api/jobs/${id}/detailed-progress`)
  }
}

// Backups API
export const backupsApi = {
  list(params = {}) {
    return apiClient.get('/api/backups', params)
  },

  get(id) {
    return apiClient.get(`/api/backups/${id}`)
  },

  restore(id, data) {
    return apiClient.post(`/api/backups/${id}/restore`, data)
  },

  async download(id) {
    const response = await fetch(`${apiClient.baseUrl}/api/backups/${id}/download`)
    if (!response.ok) {
      throw new Error(`Download failed: ${response.status}`)
    }
    return response
  },

  async downloadFile(id, filename) {
    const response = await this.download(id)
    const blob = await response.blob()
    const url = window.URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename || 'backup.tar.gz'
    document.body.appendChild(a)
    a.click()
    window.URL.revokeObjectURL(url)
    document.body.removeChild(a)
  },

  delete(id) {
    return apiClient.delete(`/api/backups/${id}`)
  },

  cleanup(days = 30) {
    return apiClient.post('/api/backups/cleanup', { days })
  },

  updateMetadata(id, metadata) {
    return apiClient.post(`/api/backups/${id}/metadata`, metadata)
  },

  // Upload backup file
  async upload(file, databaseConfigId, compressionType = 'gzip') {
    const formData = new FormData()
    formData.append('file', file)
    formData.append('database_config_id', databaseConfigId)
    formData.append('compression_type', compressionType)

    console.log('Uploading file:', file.name, 'Size:', file.size)
    console.log('Database config ID:', databaseConfigId)
    console.log('Compression type:', compressionType)

    try {
      const response = await fetch(`${apiClient.baseUrl}/api/backups/upload`, {
        method: 'POST',
        body: formData,
        // Don't set Content-Type header - let the browser set it with boundary
      })

      console.log('Response status:', response.status)
      console.log('Response headers:', Object.fromEntries(response.headers.entries()))

      if (!response.ok) {
        const errorText = await response.text()
        console.error('Upload failed - response text:', errorText)
        
        let errorData
        try {
          errorData = JSON.parse(errorText)
        } catch (e) {
          errorData = { error: errorText || 'Unknown error' }
        }
        
        throw new Error(errorData.error || `HTTP ${response.status}`)
      }

      const result = await response.json()
      console.log('Upload successful:', result)
      return result
    } catch (error) {
      console.error('Upload error:', error)
      throw error
    }
  }
}

// System API
export const systemApi = {
  info() {
    return apiClient.get('/api/system')
  },

  health() {
    return apiClient.get('/api/health')
  },

  // System API
  async getSystemInfo() {
    try {
      const response = await apiClient.request('/api/system/info')
      return response
    } catch (error) {
      console.error('Failed to fetch system info:', error)
      return { success: false, error: error.message }
    }
  },

  async getVersionInfo() {
    try {
      const response = await apiClient.request('/api/system/version')
      return response
    } catch (error) {
      console.error('Failed to fetch version info:', error)
      return { success: false, error: error.message }
    }
  },

  async getHealthStatus() {
    try {
      const response = await apiClient.request('/api/system/health')
      return response
    } catch (error) {
      console.error('Failed to fetch health status:', error)
      return { success: false, error: error.message }
    }
  },

  async getMyDumperVersion() {
    try {
      const response = await apiClient.request('/api/system/mydumper/version')
      return response
    } catch (error) {
      console.error('Failed to fetch mydumper version:', error)
      return { success: false, error: error.message }
    }
  },

  async getMyLoaderVersion() {
    try {
      const response = await apiClient.request('/api/system/myloader/version')
      return response
    } catch (error) {
      console.error('Failed to fetch myloader version:', error)
      return { success: false, error: error.message }
    }
  },

  async getWorkerStatus() {
    try {
      const response = await apiClient.request('/api/system/worker')
      return response
    } catch (error) {
      console.error('Failed to fetch worker status:', error)
      return { success: false, error: error.message }
    }
  }
}

// Logs API - system logs
export const logsApi = {
  async list(params = {}) {
    try {
      const queryParams = new URLSearchParams()
      if (params.page) queryParams.append('page', params.page)
      if (params.limit) queryParams.append('limit', params.limit)
      if (params.log_type) queryParams.append('log_type', params.log_type)
      if (params.level) queryParams.append('level', params.level)
      if (params.entity_type) queryParams.append('entity_type', params.entity_type)
      if (params.entity_id) queryParams.append('entity_id', params.entity_id)
      
      const queryString = queryParams.toString()
      const endpoint = queryString ? `/api/logs?${queryString}` : '/api/logs'
      const response = await apiClient.request(endpoint)
      return response
    } catch (error) {
      console.error('Failed to fetch logs:', error)
      return { success: false, data: [], total: 0 }
    }
  }
}

// Dashboard API - aggregated data
export const dashboardApi = {
  async getStats() {
    try {
      const response = await apiClient.request('/api/dashboard/stats')
      return response
    } catch (error) {
      console.error('Failed to fetch dashboard stats:', error)
      return { 
        success: false, 
        data: { databases: 0, activeTasks: 0, recentBackups: 0, runningJobs: 0, backupFiles: 0, tasks: 0, totalJobs: 0 } 
      }
    }
  },

  async getRecentBackups() {
    try {
      const response = await apiClient.request('/api/dashboard/recent-backups')
      return response
    } catch (error) {
      console.error('Failed to fetch recent backups:', error)
      return { success: false, data: [] }
    }
  },

  async getNextTasks() {
    try {
      const response = await apiClient.request('/api/dashboard/next-tasks')
      return response
    } catch (error) {
      console.error('Failed to fetch next tasks:', error)
      return { success: false, data: [] }
    }
  },

  async getRecentJobs(limit = 10) {
    try {
      const response = await jobsApi.list({ limit, sort: 'created_at', order: 'desc' })
      return response
    } catch (error) {
      console.error('Failed to fetch recent jobs:', error)
      return { success: false, data: [] }
    }
  }
}

export default apiClient