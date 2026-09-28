<template>
  <div class="form-data-editor">
    <div class="fd-head">
      <span class="col-check" />
      <span class="col-key">KEY</span>
      <span class="col-type">TYPE</span>
      <span class="col-val">VALUE</span>
      <span class="col-del" />
    </div>
    <div v-for="(row, index) in localRows" :key="index" class="fd-row">
      <el-checkbox v-model="row.enabled" class="col-check" @change="emitChange" />
      <el-input
        v-model="row.key"
        size="mini"
        placeholder="Key"
        class="col-key"
        @input="emitChange"
      />
      <el-select
        v-model="row.type"
        size="mini"
        class="col-type"
        @change="onTypeChange(row)"
      >
        <el-option label="Text" value="text" />
        <el-option label="File" value="file" />
      </el-select>

      <div class="col-val">
        <el-input
          v-if="row.type !== 'file'"
          v-model="row.value"
          size="mini"
          placeholder="Value"
          @input="emitChange"
        />
        <div v-else class="file-cell">
          <span class="file-name" :title="row.file_path || ''">
            {{ row.file_name || '未选择文件' }}
          </span>
          <button type="button" class="pick-btn" @click="pickFile(index)">选择文件</button>
          <button
            v-if="row.file_path"
            type="button"
            class="clear-btn"
            title="清除"
            @click="clearFile(index)"
          >
            <at-icon name="close" :size="12" />
          </button>
        </div>
      </div>

      <button type="button" class="col-del del-btn" @click="removeRow(index)">
        <at-icon name="close" :size="13" />
      </button>
    </div>
    <button type="button" class="add-btn" @click="addRow">
      <at-icon name="plus" :size="13" /> Add
    </button>
    <input
      ref="fileInput"
      type="file"
      class="hidden-file"
      @change="onFilePicked"
    >
  </div>
</template>

<script>
import AtIcon from './AtIcon'

function emptyRow() {
  return {
    key: '',
    value: '',
    enabled: true,
    type: 'text',
    file_path: '',
    file_name: ''
  }
}

function normalizeRow(row) {
  return {
    key: (row && row.key) || '',
    value: (row && row.value) || '',
    enabled: row && row.enabled !== false,
    type: row && row.type === 'file' ? 'file' : 'text',
    file_path: (row && row.file_path) || '',
    file_name: (row && row.file_name) || ''
  }
}

export default {
  name: 'FormDataEditor',
  components: { AtIcon },
  props: {
    value: {
      type: Array,
      default: () => []
    }
  },
  data() {
    return {
      localRows: [emptyRow()],
      pickIndex: -1
    }
  },
  watch: {
    value: {
      immediate: true,
      deep: true,
      handler(val) {
        if (!Array.isArray(val) || !val.length) {
          this.localRows = [emptyRow()]
          return
        }
        this.localRows = val.map(normalizeRow)
      }
    }
  },
  methods: {
    emitChange() {
      this.$emit('input', this.localRows.map(normalizeRow))
    },
    addRow() {
      this.localRows.push(emptyRow())
      this.emitChange()
    },
    removeRow(index) {
      this.localRows.splice(index, 1)
      if (!this.localRows.length) this.localRows.push(emptyRow())
      this.emitChange()
    },
    onTypeChange(row) {
      if (row.type === 'text') {
        row.file_path = ''
        row.file_name = ''
      } else {
        row.value = ''
      }
      this.emitChange()
    },
    pickFile(index) {
      this.pickIndex = index
      const input = this.$refs.fileInput
      if (!input) return
      input.value = ''
      input.click()
    },
    clearFile(index) {
      const row = this.localRows[index]
      if (!row) return
      row.file_path = ''
      row.file_name = ''
      row.value = ''
      this.emitChange()
    },
    onFilePicked(e) {
      const file = e.target.files && e.target.files[0]
      const index = this.pickIndex
      this.pickIndex = -1
      if (!file || index < 0 || !this.localRows[index]) return

      // Tauri WebView exposes absolute path on File; browser preview has no path.
      const path = file.path || ''
      if (!path) {
        this.$message && this.$message.warning
          ? this.$message.warning('当前环境无法获取文件路径，请使用桌面应用选择文件')
          : null
        return
      }
      const row = this.localRows[index]
      row.type = 'file'
      row.file_path = path
      row.file_name = file.name || path.split(/[/\\]/).pop() || 'file'
      row.value = row.file_name
      this.emitChange()
    }
  }
}
</script>

<style lang="scss" scoped>
.form-data-editor {
  font-size: 12px;
}

.fd-head,
.fd-row {
  display: grid;
  grid-template-columns: 28px minmax(100px, 1.1fr) 88px minmax(160px, 2fr) 28px;
  gap: 6px;
  align-items: center;
  padding: 4px 0;
}

.fd-head {
  color: var(--at-text-muted, #9ca3af);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  border-bottom: 1px solid var(--at-border-soft, #eef0f3);
  margin-bottom: 4px;
  padding-bottom: 6px;
}

.fd-row {
  ::v-deep .el-input__inner {
    background-color: var(--at-input-bg, #fff);
    border-color: var(--at-input-border, #dcdfe6);
    color: var(--at-input-color, #1f2937);

    &::placeholder {
      color: var(--at-input-placeholder, #9ca3af);
    }
  }
}

.col-type {
  width: 100%;
}

.file-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--at-input-color, #374151);
  background: var(--at-surface-muted, #f9fafb);
  border: 1px solid var(--at-input-border, #e5e7eb);
  border-radius: 3px;
  padding: 0 8px;
  height: 28px;
  line-height: 28px;
}

.pick-btn {
  flex-shrink: 0;
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--at-input-border, #e5e7eb);
  background: var(--at-input-bg, #fff);
  border-radius: 3px;
  cursor: pointer;
  color: var(--at-input-color, #374151);
  font-size: 12px;
  &:hover {
    border-color: #ff6c37;
    color: #ff6c37;
  }
}

.clear-btn,
.del-btn {
  border: none;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  padding: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  &:hover { color: #ef4444; }
}

.add-btn {
  margin-top: 8px;
  border: 1px dashed #d1d5db;
  background: transparent;
  color: #6b7280;
  border-radius: 4px;
  padding: 6px 10px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  &:hover {
    border-color: #ff6c37;
    color: #ff6c37;
  }
}

.hidden-file {
  display: none;
}
</style>
