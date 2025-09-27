<template>
  <div>
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-base-content">Backups</h1>
      <p class="text-base-content/70 mt-2">Browse and restore your database backups</p>
    </div>

    <div class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>Backup Name</th>
                <th>Database</th>
                <th>Size</th>
                <th>Created</th>
                <th>Compression</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>production_20240927_020000</td>
                <td>Production DB</td>
                <td>2.3 GB</td>
                <td>2 hours ago</td>
                <td><div class="badge badge-info">gzip</div></td>
                <td>
                  <div class="join">
                    <button class="btn btn-sm btn-primary">Restore</button>
                    <button class="btn btn-sm btn-ghost">Download</button>
                    <button class="btn btn-sm btn-error">Delete</button>
                  </div>
                </td>
              </tr>
              <tr>
                <td>staging_20240927_010000</td>
                <td>Staging DB</td>
                <td>1.8 GB</td>
                <td>3 hours ago</td>
                <td><div class="badge badge-info">zstd</div></td>
                <td>
                  <div class="join">
                    <button class="btn btn-sm btn-primary">Restore</button>
                    <button class="btn btn-sm btn-ghost">Download</button>
                    <button class="btn btn-sm btn-error">Delete</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
// TODO: Implement backup management logic
</script>