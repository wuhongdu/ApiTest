<template>
  <div class="kv-editor">
    <div class="kv-head">
      <span class="col-check" />
      <span class="col-key">KEY</span>
      <span class="col-val">VALUE</span>
      <span class="col-del" />
    </div>
    <div v-for="(row, index) in localRows" :key="index" class="kv-row">
      <el-checkbox v-model="row.enabled" class="col-check" @change="emitChange" />

      <el-autocomplete
        v-if="keySuggestions.length"
        v-model="row.key"
        size="mini"
        class="col-key"
        placeholder="Key"
        :fetch-suggestions="queryKeySuggestions"
        :trigger-on-focus="true"
        @select="() => onKeySelect(row)"
        @input="emitChange"
      />
      <el-input
        v-else
        v-model="row.key"
        size="mini"
        placeholder="Key"
        class="col-key"
        @input="emitChange"
      />

      <el-autocomplete
        v-if="valueOptionsFor(row.key).length"
        v-model="row.value"
        size="mini"
        class="col-val"
        placeholder="Value"
        :fetch-suggestions="(q, cb) => queryValueSuggestions(row.key, q, cb)"
        :trigger-on-focus="true"
        @input="emitChange"
      />
      <el-input
        v-else
        v-model="row.value"
        size="mini"
        placeholder="Value"
        class="col-val"
        @input="emitChange"
      />

      <button type="button" class="col-del del-btn" @click="removeRow(index)">
        <at-icon name="close" :size="13" />
      </button>
    </div>
    <button type="button" class="add-btn" @click="addRow">
      <at-icon name="plus" :size="13" /> Add
    </button>
  </div>
</template>

<script>
import AtIcon from './AtIcon'

function emptyRow() {
  return { key: '', value: '', enabled: true }
}

export default {
  name: 'KvEditor',
  components: { AtIcon },
  props: {
    value: {
      type: Array,
      default: () => []
    },
    /** Suggested keys, e.g. common HTTP headers */
    keySuggestions: {
      type: Array,
      default: () => []
    },
    /** Map of key(lower) -> value suggestion list, e.g. Content-Type options */
    valueSuggestions: {
      type: Object,
      default: () => ({})
    }
  },
  data() {
    return {
      localRows: []
    }
  },
  watch: {
    value: {
      immediate: true,
      deep: true,
      handler(val) {
        const rows = Array.isArray(val) ? val.map(r => ({
          key: r.key || '',
          value: r.value || '',
          enabled: r.enabled !== false
        })) : []
        if (!rows.length) rows.push(emptyRow())
        this.localRows = rows
      }
    }
  },
  methods: {
    emitChange() {
      this.$emit('input', this.localRows.map(r => ({ ...r })))
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
    queryKeySuggestions(query, cb) {
      const q = (query || '').toLowerCase()
      const list = (this.keySuggestions || [])
        .filter(k => !q || String(k).toLowerCase().includes(q))
        .map(k => ({ value: k }))
      cb(list)
    },
    valueOptionsFor(key) {
      if (!key) return []
      const map = this.valueSuggestions || {}
      const lower = String(key).toLowerCase()
      if (map[key]) return map[key]
      if (map[lower]) return map[lower]
      const found = Object.keys(map).find(k => k.toLowerCase() === lower)
      return found ? map[found] : []
    },
    queryValueSuggestions(key, query, cb) {
      const q = (query || '').toLowerCase()
      const list = this.valueOptionsFor(key)
        .filter(v => !q || String(v).toLowerCase().includes(q))
        .map(v => ({ value: v }))
      cb(list)
    },
    onKeySelect(row) {
      // if value empty and key has a default first suggestion, leave empty for user pick
      this.emitChange()
      this.$forceUpdate()
    }
  }
}
</script>

<style lang="scss" scoped>
.kv-editor {
  font-size: 12px;
}

.kv-head,
.kv-row {
  display: grid;
  grid-template-columns: 28px 1fr 1fr 28px;
  gap: 6px;
  align-items: center;
}

.kv-head {
  padding: 4px 0 8px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: var(--at-text-muted, #9ca3af);
  border-bottom: 1px solid var(--at-border-soft, #eef0f3);
  margin-bottom: 6px;
}

.kv-row {
  margin-bottom: 6px;

  ::v-deep .el-input__inner {
    border-radius: 3px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
    background-color: var(--at-input-bg, #fff);
    border-color: var(--at-input-border, #dcdfe6);
    color: var(--at-input-color, #1f2937);

    &::placeholder {
      color: var(--at-input-placeholder, #9ca3af);
    }
  }
}

.del-btn {
  border: none;
  background: transparent;
  color: #c0c4cc;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;

  &:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.08);
  }
}

.add-btn {
  border: none;
  background: transparent;
  color: #ff6c37;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  padding: 6px 0;
  display: inline-flex;
  align-items: center;
  gap: 4px;

  &:hover { color: #ff5722; }
}
</style>
