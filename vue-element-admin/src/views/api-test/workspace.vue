<template>
  <div
    class="apitest-workspace"
    :class="{ 'theme-dark': theme === 'dark' }"
    tabindex="0"
    @keydown="onKeydown"
  >
    <div v-if="!isTauri" class="banner">
      <el-alert
        type="warning"
        :closable="false"
        show-icon
        title="请使用 npm run tauri:dev 启动桌面应用，浏览器预览无法发送请求或读写 SQLite。"
      />
    </div>

    <!-- Unified title bar: brand + menus + window controls -->
    <nav class="titlebar" @click="closeMenus">
      <div class="titlebar-left" @mousedown="onTitleDrag">
        <img class="brand-mark sm" src="/apitest-icon.png" alt="ApiTest" />
        <span class="brand-name">ApiTest</span>
        <div class="menubar-menus">
          <div
            v-for="menu in menus"
            :key="menu.key"
            class="menu-item"
            :class="{ open: openMenu === menu.key }"
            @click.stop="toggleMenu(menu.key)"
            @mouseenter="onMenuEnter(menu.key)"
            @mousedown.stop
          >
            <span class="menu-label">{{ menu.label }}</span>
            <div v-show="openMenu === menu.key" class="menu-dropdown" @click.stop>
              <template v-for="(item, idx) in menu.items">
                <div
                  v-if="item.type === 'divider'"
                  :key="menu.key + '-d-' + idx"
                  class="menu-divider"
                />
                <button
                  v-else
                  :key="menu.key + '-' + idx"
                  class="menu-option"
                  :disabled="item.disabled && item.disabled()"
                  @click="onMenuAction(item)"
                >
                  <span>{{ item.label }}</span>
                  <span v-if="item.shortcut" class="menu-shortcut">{{ item.shortcut }}</span>
                </button>
              </template>
            </div>
          </div>
        </div>
      </div>

      <div class="titlebar-drag" @mousedown="onTitleDrag" @dblclick="onWinMaximize" />

      <div class="titlebar-right" @mousedown.stop>
        <button
          v-if="isTauri"
          class="win-btn"
          title="最小化"
          @click="onWinMinimize"
        >
          <at-icon name="win-min" :size="12" />
        </button>
        <button
          v-if="isTauri"
          class="win-btn"
          :title="winMaximized ? '还原' : '最大化'"
          @click="onWinMaximize"
        >
          <at-icon :name="winMaximized ? 'win-restore' : 'win-max'" :size="12" />
        </button>
        <button
          v-if="isTauri"
          class="win-btn win-close"
          title="关闭"
          @click="onWinClose"
        >
          <at-icon name="win-close" :size="12" />
        </button>
      </div>
    </nav>

    <!-- Tool chrome -->
    <header class="topbar">
      <div class="topbar-center">
        <el-select
          v-model="activeWorkspaceId"
          size="mini"
          class="tb-select ws-select"
          placeholder="工作区"
          @change="onWorkspaceChange"
        >
          <el-option
            v-for="ws in workspaces"
            :key="ws.id"
            :label="ws.name"
            :value="ws.id"
          />
        </el-select>
        <button class="icon-btn" title="新建工作区" @click="onAddWorkspace">
          <at-icon name="plus" :size="15" />
        </button>

        <span class="tb-divider" />

        <at-icon name="env" :size="15" class="tb-env-icon" />
        <el-select
          v-model="activeEnvId"
          size="mini"
          class="tb-select env-select"
          placeholder="No Environment"
          @change="onEnvChange"
        >
          <el-option
            v-for="env in environments"
            :key="env.id"
            :label="envDisplayName(env.name)"
            :value="env.id"
          />
        </el-select>
        <button class="icon-btn" title="环境变量" @click="envDialogVisible = true">
          <at-icon name="env" :size="15" />
        </button>
      </div>

      <div class="topbar-right">
        <button class="ghost-btn" @click="importDialogVisible = true">
          <at-icon name="import" :size="14" /> 导入
        </button>
        <button
          class="ghost-btn"
          :disabled="!selectedCollectionId"
          @click="onExportCollection"
        >
          <at-icon name="export" :size="14" /> 导出
        </button>
        <button
          class="ghost-btn"
          :disabled="!selectedCollectionId"
          title="导出为 JMeter .jmx"
          @click="jmeterExportDialogVisible = true"
        >
          JMeter
        </button>
        <span class="tb-divider" />
        <button class="ghost-btn" title="设置" @click="openSettings">
          <at-icon name="settings" :size="14" /> 设置
        </button>
        <button
          class="icon-btn theme-toggle"
          :title="themeDark ? '切换亮色' : '切换暗色'"
          @click="toggleTheme"
        >
          <at-icon :name="themeDark ? 'sun' : 'moon'" :size="15" />
        </button>
        <span class="runtime-chip" :class="{ ok: isTauri }">
          {{ isTauri ? 'Desktop' : 'Browser' }}
        </span>
      </div>
    </header>

    <div class="main-panes">
      <!-- Left rail -->
      <aside class="pane-left">
        <div class="left-seg">
          <button
            class="seg-btn"
            :class="{ active: leftTab === 'collections' }"
            @click="leftTab = 'collections'"
          >
            <at-icon name="folder" :size="13" />
            Collections
          </button>
          <button
            class="seg-btn"
            :class="{ active: leftTab === 'history' }"
            @click="leftTab = 'history'"
          >
            <at-icon name="history" :size="13" />
            History
          </button>
        </div>

        <template v-if="leftTab === 'collections'">
          <div class="pane-header">
            <span class="pane-title">Collections</span>
            <div class="pane-actions">
              <button class="icon-btn sm" title="新建集合" @click="onAddCollection">
                <at-icon name="folder-plus" :size="15" />
              </button>
              <button class="icon-btn sm" title="新建请求" @click="onAddRequest">
                <at-icon name="request" :size="15" />
              </button>
              <button class="icon-btn sm" title="刷新" @click="loadTree">
                <at-icon name="refresh" :size="15" />
              </button>
            </div>
          </div>
          <el-tree
            ref="tree"
            class="tree"
            :data="treeData"
            node-key="id"
            highlight-current
            default-expand-all
            :expand-on-click-node="false"
            @node-click="onNodeClick"
          >
            <span slot-scope="{ data }" class="tree-node">
              <span class="tree-label">
                <span
                  v-if="data.node_type === 'request'"
                  class="method-badge"
                  :class="'m-' + (data.method || 'get').toLowerCase()"
                >{{ shortMethod(data.method) }}</span>
                <at-icon v-else name="folder" :size="14" class="folder-icon" />
                <span class="tree-text">{{ data.label }}</span>
              </span>
              <span class="tree-actions" @click.stop>
                <button
                  v-if="data.node_type === 'collection'"
                  class="icon-btn xs"
                  title="编辑集合"
                  @click="onEditCollection(data)"
                >
                  <at-icon name="edit" :size="13" />
                </button>
                <button class="icon-btn xs danger" @click="onDeleteNode(data)">
                  <at-icon name="trash" :size="13" />
                </button>
              </span>
            </span>
          </el-tree>
        </template>

        <template v-else>
          <div class="pane-header">
            <span class="pane-title">History</span>
            <div class="pane-actions">
              <button class="icon-btn sm" title="刷新" @click="loadHistory">
                <at-icon name="refresh" :size="15" />
              </button>
              <button class="icon-btn sm danger" title="清空" @click="onClearHistory">
                <at-icon name="trash" :size="15" />
              </button>
            </div>
          </div>
          <div class="history-list">
            <div
              v-for="item in historyList"
              :key="item.id"
              class="history-item"
              @click="onOpenHistory(item)"
            >
              <div class="history-top">
                <span
                  class="method-badge"
                  :class="'m-' + (item.method || 'get').toLowerCase()"
                >{{ shortMethod(item.method) }}</span>
                <span class="status-chip" :class="statusClass(item.status_code)">
                  {{ item.status_code || '-' }}
                </span>
                <span class="duration">{{ item.duration_ms || 0 }}ms</span>
              </div>
              <div class="history-url" :title="item.url">{{ item.url }}</div>
              <div class="history-time">{{ item.created_at }}</div>
            </div>
            <div v-if="!historyList.length" class="history-empty">暂无历史记录</div>
          </div>
        </template>
      </aside>

      <!-- Main editor -->
      <section class="pane-right">
        <div v-if="!currentRequest" class="empty">
          <div class="empty-illu">
            <at-icon name="api" :size="28" />
          </div>
          <p class="empty-title">创建一个请求开始调试</p>
          <p class="empty-sub">从左侧 Collections 新建，或导入已有集合</p>
          <button class="send-btn empty-cta" @click="onAddRequest">New Request</button>
        </div>

        <template v-else>
          <div class="req-title-row">
            <el-input
              v-model="currentRequest.name"
              class="req-name"
              size="small"
              placeholder="Request name"
            />
            <span class="shortcut-hint">Ctrl+Enter Send · Ctrl+S Save</span>
          </div>

          <!-- Postman-style URL bar -->
          <div class="url-bar-wrap">
            <div class="url-bar">
              <el-select
                v-model="currentRequest.method"
                class="method-select"
                size="small"
                :class="'method-' + (currentRequest.method || 'GET').toLowerCase()"
              >
                <el-option v-for="m in methods" :key="m" :label="m" :value="m">
                  <span class="method-badge" :class="'m-' + m.toLowerCase()">{{ m }}</span>
                </el-option>
              </el-select>
              <el-input
                :value="urlInputDisplay"
                class="url-input"
                size="small"
                :placeholder="urlPlaceholder"
                @input="onUrlInputChange"
                @keyup.enter.native="onSend"
              />
              <button
                v-if="sending"
                class="cancel-btn"
                title="取消当前请求"
                @click="onCancelSend"
              >
                <at-icon name="close" :size="14" />
                <span>Cancel</span>
              </button>
              <button
                v-else
                class="send-btn"
                @click="onSend"
              >
                <at-icon name="send" :size="14" />
                <span>Send</span>
              </button>
              <button class="save-btn" :disabled="saving" @click="onSave">
                <at-icon v-if="saving" name="loading" :size="14" />
                <span v-else>Save</span>
              </button>
              <button
                class="loadtest-btn"
                :disabled="!currentRequest || loadTesting"
                title="内置压测"
                @click="openLoadTest"
              >
                压测
              </button>
            </div>
          </div>

          <div ref="editorSplit" class="editor-split" :class="{ 'is-splitting': splitting }">
            <div class="req-section" :style="reqSectionStyle">
              <el-tabs v-model="reqTab" class="req-tabs">
                <el-tab-pane label="Params" name="params">
                  <kv-editor v-model="currentRequest.params" />
                </el-tab-pane>
                <el-tab-pane label="Headers" name="headers">
                  <kv-editor
                    v-model="currentRequest.headers"
                    :key-suggestions="commonHeaderKeys"
                    :value-suggestions="commonHeaderValues"
                  />
                </el-tab-pane>
                <el-tab-pane label="Body" name="body">
                  <div class="body-toolbar">
                    <el-radio-group
                      v-model="currentRequest.body_type"
                      size="mini"
                      class="body-type-group"
                      @change="onBodyTypeChange"
                    >
                      <el-radio label="none">none</el-radio>
                      <el-radio label="form-data">form-data</el-radio>
                      <el-radio label="x-www-form-urlencoded">x-www-form-urlencoded</el-radio>
                      <el-radio label="raw">raw</el-radio>
                      <el-radio label="binary">binary</el-radio>
                      <el-radio label="graphql">GraphQL</el-radio>
                    </el-radio-group>
                    <el-select
                      v-if="currentRequest.body_type === 'raw'"
                      v-model="currentRequest.body_language"
                      size="mini"
                      class="raw-lang-select"
                      @change="onRawLanguageChange"
                    >
                      <el-option
                        v-for="lang in rawLanguages"
                        :key="lang.value"
                        :label="lang.label"
                        :value="lang.value"
                      />
                    </el-select>
                    <button
                      v-if="canBeautifyBody"
                      type="button"
                      class="beautify-btn"
                      @click="beautifyBody"
                    >Beautify</button>
                  </div>

                  <div v-if="currentRequest.body_type === 'none'" class="body-none">
                    This request does not have a body
                  </div>

                  <form-data-editor
                    v-else-if="currentRequest.body_type === 'form-data'"
                    v-model="formBodyRows"
                    @input="syncFormBodyToContent"
                  />

                  <kv-editor
                    v-else-if="currentRequest.body_type === 'x-www-form-urlencoded'"
                    v-model="formBodyRows"
                    @input="syncFormBodyToContent"
                  />

                  <div v-else-if="currentRequest.body_type === 'binary'" class="binary-body">
                    <div class="binary-file-row">
                      <span class="binary-name" :title="binaryFilePath || ''">
                        {{ binaryFileName || '未选择文件' }}
                      </span>
                      <button type="button" class="pick-btn" @click="pickBinaryFile">选择文件</button>
                      <button
                        v-if="binaryFilePath"
                        type="button"
                        class="clear-btn"
                        title="清除"
                        @click="clearBinaryFile"
                      >
                        <at-icon name="close" :size="12" />
                      </button>
                    </div>
                    <input
                      ref="binaryFileInput"
                      type="file"
                      class="hidden-file"
                      @change="onBinaryFilePicked"
                    >
                  </div>

                  <div v-else-if="currentRequest.body_type === 'graphql'" class="graphql-body">
                    <div class="label">Query</div>
                    <el-input
                      v-model="graphqlQuery"
                      type="textarea"
                      :rows="8"
                      class="body-textarea"
                      placeholder="query { ... }"
                      @input="syncGraphqlToContent"
                    />
                    <div class="label" style="margin-top: 10px">GraphQL Variables</div>
                    <el-input
                      v-model="graphqlVariables"
                      type="textarea"
                      :rows="5"
                      class="body-textarea"
                      placeholder="{ }"
                      @input="syncGraphqlToContent"
                    />
                  </div>

                  <el-input
                    v-else
                    v-model="currentRequest.body_content"
                    type="textarea"
                    :rows="10"
                    :placeholder="bodyPlaceholder"
                    class="body-textarea"
                  />
                </el-tab-pane>
                <el-tab-pane label="Pre-request" name="pre">
                  <div class="script-hint">发送前执行 · at.env / at.request / at.expect</div>
                  <el-input
                    v-model="currentRequest.pre_script"
                    type="textarea"
                    :rows="10"
                    class="body-textarea"
                    placeholder="// at.env.set('token', 'xxx')"
                  />
                </el-tab-pane>
                <el-tab-pane label="Tests" name="tests">
                  <div class="script-hint">响应后断言 · at.response / at.test / at.expect</div>
                  <el-input
                    v-model="currentRequest.test_script"
                    type="textarea"
                    :rows="10"
                    class="body-textarea"
                    placeholder="at.test('status 200', () => at.expect(at.response.status).toBe(200));"
                  />
                </el-tab-pane>
                <el-tab-pane label="Mock" name="mock">
                  <div class="mock-bar">
                    <el-switch v-model="currentRequest.mock_enabled" active-text="启用 Mock" />
                    <span class="label">Status</span>
                    <el-input-number
                      v-model="currentRequest.mock_status"
                      size="mini"
                      :min="100"
                      :max="599"
                    />
                    <span class="label">Delay(ms)</span>
                    <el-input-number
                      v-model="currentRequest.mock_delay_ms"
                      size="mini"
                      :min="0"
                      :max="30000"
                      :step="100"
                    />
                  </div>
                  <div class="script-hint">启用后 Send 不再打真实网络</div>
                  <div class="label" style="margin: 8px 0">Headers</div>
                  <kv-editor
                    v-model="currentRequest.mock_headers"
                    :key-suggestions="commonHeaderKeys"
                    :value-suggestions="commonHeaderValues"
                  />
                  <div class="label" style="margin: 8px 0">Body</div>
                  <el-input
                    v-model="currentRequest.mock_body"
                    type="textarea"
                    :rows="8"
                    placeholder="mock response body"
                    class="body-textarea"
                  />
                </el-tab-pane>
              </el-tabs>
            </div>

            <div
              class="split-resizer"
              title="拖动调整高度 · 双击复位"
              @mousedown="onSplitMouseDown"
              @dblclick="resetSplitPercent"
            >
              <span class="split-grip" />
            </div>

            <div class="resp-section">
              <div class="response-meta">
                <span class="resp-label">Response</span>
                <template v-if="sending">
                  <span class="meta-pill sending-pill">
                    <at-icon name="loading" :size="12" />
                    等待中…
                  </span>
                </template>
                <template v-else-if="response">
                  <span v-if="response.mocked" class="mock-chip">MOCK</span>
                  <span
                    class="status-big"
                    :class="response.status_text === 'CANCELLED' ? 's-warn' : statusClass(response.status)"
                  >
                    {{ response.status || '—' }}
                    <em>{{ response.status_text }}</em>
                  </span>
                  <span class="meta-pill" title="耗时">{{ formatDuration(response.duration_ms) }}</span>
                  <span class="meta-pill" title="响应体大小">{{ formatBytes(responseBodyBytes) }}</span>
                  <span
                    v-if="responseContentType"
                    class="meta-pill type-pill"
                    :title="responseContentType"
                  >{{ shortContentType(responseContentType) }}</span>
                  <span class="meta-spacer" />
                  <button
                    type="button"
                    class="meta-icon-btn"
                    title="复制响应体"
                    :disabled="!response.body"
                    @click="copyText(response.body, '已复制响应体')"
                  >
                    <at-icon name="copy" :size="13" />
                  </button>
                </template>
                <span v-else class="meta-muted">未发送</span>
              </div>

              <div v-if="response && response.url" class="response-url" :title="response.url">
                <span class="url-label">URL</span>
                <span class="url-text">{{ response.url }}</span>
                <button
                  type="button"
                  class="meta-icon-btn"
                  title="复制 URL"
                  @click="copyText(response.url, '已复制 URL')"
                >
                  <at-icon name="copy" :size="12" />
                </button>
              </div>

              <el-alert
                v-if="response && response.error"
                :type="response.status_text === 'CANCELLED' ? 'warning' : 'error'"
                :closable="false"
                :title="response.error"
                show-icon
                class="resp-error"
              />

              <div v-if="sending" class="response-empty response-loading">
                <at-icon name="loading" :size="28" />
                <p>正在等待响应…</p>
                <span class="loading-hint">可点击 Cancel 取消</span>
              </div>

              <el-tabs v-else-if="response" v-model="respTab" class="resp-tabs">
                <el-tab-pane label="Body" name="body">
                  <json-viewer :text="response.body || ''" />
                </el-tab-pane>
                <el-tab-pane :label="headersTabLabel" name="headers">
                  <div v-if="!(response.headers && response.headers.length)" class="response-empty compact">
                    无响应头
                  </div>
                  <div v-else class="headers-table">
                    <div class="headers-head">
                      <span>Name</span>
                      <span>Value</span>
                    </div>
                    <div
                      v-for="(h, i) in response.headers"
                      :key="i"
                      class="headers-row"
                    >
                      <span class="h-key" :title="h.key">{{ h.key }}</span>
                      <span class="h-val" :title="h.value">{{ h.value }}</span>
                      <button
                        type="button"
                        class="meta-icon-btn"
                        title="复制"
                        @click="copyText(h.key + ': ' + h.value, '已复制')"
                      >
                        <at-icon name="copy" :size="12" />
                      </button>
                    </div>
                  </div>
                </el-tab-pane>
                <el-tab-pane :label="testTabLabel" name="tests">
                  <div v-if="!testResults.length" class="response-empty compact">
                    无测试结果（可在 Tests 页编写断言）
                  </div>
                  <div
                    v-for="(t, i) in testResults"
                    :key="i"
                    class="test-row"
                    :class="{ pass: t.passed, fail: !t.passed }"
                  >
                    <at-icon :name="t.passed ? 'pass' : 'fail'" :size="15" />
                    <strong>{{ t.name }}</strong>
                    <span v-if="!t.passed" class="test-err">{{ t.error }}</span>
                  </div>
                </el-tab-pane>
              </el-tabs>

              <div v-else class="response-empty">
                <at-icon name="send" :size="28" />
                <p>点击 Send 查看响应</p>
              </div>
            </div>
          </div>
        </template>
      </section>
    </div>

    <el-dialog
      :title="collectionForm.id ? '编辑集合' : '新建集合'"
      :visible.sync="collectionDialogVisible"
      width="480px"
      custom-class="apitest-dlg"
    >
      <el-form label-position="top" size="small" @submit.native.prevent>
        <el-form-item label="名称" required>
          <el-input
            v-model="collectionForm.name"
            placeholder="集合名称"
            maxlength="120"
            @keyup.enter.native="saveCollection"
          />
        </el-form-item>
        <el-form-item label="全局 URL 前缀">
          <el-input
            v-model="collectionForm.base_url"
            placeholder="https://api.example.com/v1 或环境变量前缀"
            clearable
          />
          <div class="env-hint" style="margin-top: 8px; margin-bottom: 0">
            请求 URL 为相对路径时自动拼接；以 http:// / https:// 开头则忽略前缀。支持 <code v-text="varExample" />。
          </div>
        </el-form-item>
      </el-form>
      <span slot="footer">
        <el-button @click="collectionDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="collectionSaving" @click="saveCollection">保存</el-button>
      </span>
    </el-dialog>

    <el-dialog
      title="导出 JMeter (.jmx)"
      :visible.sync="jmeterExportDialogVisible"
      width="440px"
      custom-class="apitest-dlg"
    >
      <div class="env-hint">将当前集合导出为 Apache JMeter 5.x 测试计划，可在 JMeter 中继续编辑与运行。</div>
      <el-form label-position="top" size="small">
        <el-form-item label="线程数 (Threads)">
          <el-input-number v-model="jmeterForm.threads" :min="1" :max="500" />
        </el-form-item>
        <el-form-item label="循环次数 (Loops)">
          <el-input-number v-model="jmeterForm.loops" :min="1" :max="10000" />
        </el-form-item>
        <el-form-item label="Ramp-up (秒)">
          <el-input-number v-model="jmeterForm.ramp_up" :min="0" :max="3600" />
        </el-form-item>
      </el-form>
      <span slot="footer">
        <el-button @click="jmeterExportDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="jmeterExporting" @click="onExportJmeter">导出 .jmx</el-button>
      </span>
    </el-dialog>

    <el-dialog
      title="内置压测"
      :visible.sync="loadTestDialogVisible"
      width="560px"
      custom-class="apitest-dlg"
    >
      <div class="env-hint">
        对当前请求做多线程压测（不写历史）。总量 threads × loops ≤ 10000。
      </div>
      <el-form label-position="top" size="small" inline class="loadtest-form">
        <el-form-item label="线程数">
          <el-input-number v-model="loadTestForm.threads" :min="1" :max="100" />
        </el-form-item>
        <el-form-item label="每线程循环">
          <el-input-number v-model="loadTestForm.loops" :min="1" :max="1000" />
        </el-form-item>
        <el-form-item label="Ramp-up (秒)">
          <el-input-number v-model="loadTestForm.ramp_up_secs" :min="0" :max="120" :step="0.5" />
        </el-form-item>
      </el-form>
      <div v-if="loadTestResult" class="loadtest-result">
        <div class="loadtest-stats">
          <div class="stat"><span class="stat-label">总量</span><strong>{{ loadTestResult.total }}</strong></div>
          <div class="stat ok"><span class="stat-label">成功</span><strong>{{ loadTestResult.success }}</strong></div>
          <div class="stat err"><span class="stat-label">失败</span><strong>{{ loadTestResult.failed }}</strong></div>
          <div class="stat"><span class="stat-label">总耗时</span><strong>{{ loadTestResult.duration_ms }} ms</strong></div>
          <div class="stat"><span class="stat-label">吞吐</span><strong>{{ formatRps(loadTestResult.throughput_rps) }} rps</strong></div>
          <div class="stat"><span class="stat-label">平均</span><strong>{{ formatMs(loadTestResult.avg_ms) }}</strong></div>
          <div class="stat"><span class="stat-label">最小</span><strong>{{ loadTestResult.min_ms }} ms</strong></div>
          <div class="stat"><span class="stat-label">最大</span><strong>{{ loadTestResult.max_ms }} ms</strong></div>
          <div class="stat"><span class="stat-label">P95</span><strong>{{ loadTestResult.p95_ms }} ms</strong></div>
        </div>
        <div v-if="loadTestResult.status_counts && loadTestResult.status_counts.length" class="status-breakdown">
          <div class="stat-label">状态码分布</div>
          <div class="status-chips">
            <span
              v-for="s in loadTestResult.status_counts"
              :key="s.status"
              class="status-chip"
              :class="statusClass(s.status)"
            >{{ s.status || 'ERR' }} × {{ s.count }}</span>
          </div>
        </div>
        <div v-if="loadTestResult.error_samples && loadTestResult.error_samples.length" class="error-samples">
          <div class="stat-label">错误样例</div>
          <ul>
            <li v-for="(e, i) in loadTestResult.error_samples" :key="i">{{ e }}</li>
          </ul>
        </div>
      </div>
      <span slot="footer">
        <el-button @click="loadTestDialogVisible = false">关闭</el-button>
        <el-button type="primary" :loading="loadTesting" @click="onRunLoadTest">开始压测</el-button>
      </span>
    </el-dialog>

    <el-dialog title="环境变量" :visible.sync="envDialogVisible" width="640px" custom-class="apitest-dlg">
      <div class="env-hint">在 URL / Header / Body 中使用 <code v-text="varExample" /> 引用变量。</div>
      <kv-editor v-model="envVarRows" />
      <span slot="footer">
        <el-button @click="envDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingEnv" @click="saveEnvVars">保存变量</el-button>
      </span>
    </el-dialog>

    <el-dialog title="导入集合" :visible.sync="importDialogVisible" width="720px" custom-class="apitest-dlg">
      <div class="env-hint">
        支持粘贴：
        <code>ApiTest JSON</code>、
        <code>Postman Collection v2</code>、
        <code>Swagger 2.0</code>、
        <code>OpenAPI 3.x</code>
        （JSON 或 YAML）
      </div>
      <el-input
        v-model="importJson"
        type="textarea"
        :rows="14"
        :placeholder="importPlaceholder"
      />
      <span slot="footer">
        <el-button @click="importDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="importing" @click="onImportCollection">导入</el-button>
      </span>
    </el-dialog>

    <el-dialog title="快捷键" :visible.sync="helpDialogVisible" width="420px" custom-class="apitest-dlg">
      <ul class="shortcut-list">
        <li><kbd>Ctrl</kbd> + <kbd>Enter</kbd><span>发送请求</span></li>
        <li><kbd>Ctrl</kbd> + <kbd>S</kbd><span>保存请求</span></li>
        <li><kbd>Ctrl</kbd> + <kbd>N</kbd><span>新建请求</span></li>
        <li><kbd>Ctrl</kbd> + <kbd>,</kbd><span>打开设置</span></li>
      </ul>
      <span slot="footer">
        <el-button type="primary" @click="helpDialogVisible = false">知道了</el-button>
      </span>
    </el-dialog>

    <el-dialog
      title="设置"
      :visible.sync="settingsDialogVisible"
      width="560px"
      custom-class="apitest-dlg settings-dlg"
    >
      <el-tabs v-model="settingsTab" class="settings-tabs">
        <el-tab-pane label="通用" name="general">
          <div class="settings-row">
            <div class="settings-label">
              <strong>外观主题</strong>
              <p>切换应用亮色 / 暗色界面</p>
            </div>
            <el-radio-group
              :value="theme"
              size="small"
              @input="onSettingsThemeChange"
            >
              <el-radio-button label="light">亮色</el-radio-button>
              <el-radio-button label="dark">暗色</el-radio-button>
            </el-radio-group>
          </div>
          <div class="settings-row">
            <div class="settings-label">
              <strong>环境变量</strong>
              <p>配置当前环境的 <code v-text="varExample" /> 变量</p>
            </div>
            <el-button size="small" @click="settingsDialogVisible = false; envDialogVisible = true">
              打开环境变量
            </el-button>
          </div>
          <div class="settings-row">
            <div class="settings-label">
              <strong>请求历史</strong>
              <p>清空本地发送历史记录</p>
            </div>
            <el-button size="small" type="danger" plain @click="onClearHistory">清空历史</el-button>
          </div>
        </el-tab-pane>
        <el-tab-pane label="数据" name="data">
          <div class="settings-row stacked">
            <div class="settings-label">
              <strong>SQLite 数据库路径</strong>
              <p>所有集合、请求、环境与历史均保存在此文件</p>
            </div>
            <code class="settings-path">{{ dbPath || '加载中…' }}</code>
          </div>
          <div class="settings-row">
            <div class="settings-label">
              <strong>Schema 版本</strong>
            </div>
            <span class="settings-meta">{{ dbVersion || '-' }}</span>
          </div>
        </el-tab-pane>
        <el-tab-pane label="关于" name="about">
          <div class="settings-about">
            <img class="brand-mark" src="/apitest-icon.png" alt="ApiTest" />
            <h3>ApiTest</h3>
            <p>本地桌面 API 调试工具</p>
            <p class="settings-meta">Vue + Tauri + SQLite · 无登录 · 数据本地存储</p>
            <el-button size="mini" @click="helpDialogVisible = true">查看快捷键</el-button>
          </div>
        </el-tab-pane>
      </el-tabs>
      <span slot="footer">
        <el-button type="primary" @click="settingsDialogVisible = false">完成</el-button>
      </span>
    </el-dialog>
  </div>
</template>

<script>
import KvEditor from './components/KvEditor'
import FormDataEditor from './components/FormDataEditor'
import JsonViewer from './components/JsonViewer'
import AtIcon from './components/AtIcon'
import { isTauriRuntime, windowMinimize, windowToggleMaximize, windowClose, windowIsMaximized, windowStartDragging } from '@/utils/tauri'
import { runPreScript, runTestScript } from '@/utils/scriptRunner'
import * as api from '@/api/apitest'

const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']
const FORM_BODY_TYPES = ['form-data', 'x-www-form-urlencoded']
const RAW_LANGUAGES = [
  { value: 'text', label: 'Text' },
  { value: 'javascript', label: 'JavaScript' },
  { value: 'json', label: 'JSON' },
  { value: 'html', label: 'HTML' },
  { value: 'xml', label: 'XML' }
]

function normalizeBodyFields(req) {
  if (!req) return req
  let bodyType = req.body_type || 'none'
  let bodyLanguage = req.body_language || ''
  if (bodyType === 'json') {
    bodyType = 'raw'
    if (!bodyLanguage) bodyLanguage = 'json'
  }
  if (bodyType === 'raw' && !bodyLanguage) bodyLanguage = 'json'
  req.body_type = bodyType
  req.body_language = bodyLanguage
  return req
}

function parseBinaryMeta(content) {
  const trimmed = String(content || '').trim()
  if (!trimmed) return { file_path: '', file_name: '' }
  if (trimmed.startsWith('{')) {
    try {
      const obj = JSON.parse(trimmed)
      return {
        file_path: obj.file_path || obj.path || '',
        file_name: obj.file_name || obj.name || ''
      }
    } catch (e) {
      return { file_path: '', file_name: '' }
    }
  }
  const parts = trimmed.split(/[/\\]/)
  return { file_path: trimmed, file_name: parts[parts.length - 1] || trimmed }
}

function parseGraphqlContent(content) {
  try {
    const obj = JSON.parse(content || '{}')
    const variables = obj.variables
    return {
      query: obj.query || '',
      variables: typeof variables === 'string'
        ? variables
        : JSON.stringify(variables == null ? {} : variables, null, 2)
    }
  } catch (e) {
    return { query: '', variables: '{}' }
  }
}

function beautifyXml(input) {
  const text = String(input || '').trim()
  if (!text) return text
  const normalized = text.replace(/>\s*</g, '>\n<')
  const lines = normalized.split('\n')
  let indent = 0
  const out = []
  lines.forEach(line => {
    const trimmed = line.trim()
    if (!trimmed) return
    if (/^<\/\w/.test(trimmed)) indent = Math.max(indent - 1, 0)
    out.push(`${'  '.repeat(indent)}${trimmed}`)
    if (/^<[^!?/][^>]*[^/]>$/.test(trimmed)) indent += 1
  })
  return out.join('\n')
}

/** Common HTTP header keys (Postman / ApiFox style suggestions) */
const COMMON_HEADER_KEYS = [
  'Accept',
  'Accept-Encoding',
  'Accept-Language',
  'Authorization',
  'Cache-Control',
  'Connection',
  'Content-Length',
  'Content-Type',
  'Cookie',
  'Host',
  'Origin',
  'Referer',
  'User-Agent',
  'X-Requested-With',
  'X-Request-ID',
  'X-API-Key',
  'X-Access-Token',
  'X-CSRF-Token',
  'If-None-Match',
  'If-Modified-Since'
]

const COMMON_HEADER_VALUES = {
  'Content-Type': [
    'application/json',
    'application/json; charset=utf-8',
    'application/x-www-form-urlencoded',
    'multipart/form-data',
    'text/plain',
    'text/html',
    'application/xml',
    'application/octet-stream'
  ],
  Accept: [
    'application/json',
    '*/*',
    'text/plain',
    'text/html',
    'application/xml'
  ],
  'Cache-Control': [
    'no-cache',
    'no-store',
    'max-age=0',
    'private',
    'public'
  ],
  Connection: ['keep-alive', 'close'],
  'X-Requested-With': ['XMLHttpRequest'],
  Authorization: [
    'Bearer {{token}}',
    'Basic {{credentials}}'
  ]
}

function emptyKv() {
  return [{ key: '', value: '', enabled: true, type: 'text', file_path: '', file_name: '' }]
}

function emptyFormRow() {
  return [{ key: '', value: '', enabled: true, type: 'text', file_path: '', file_name: '' }]
}

function parseFormRows(content) {
  try {
    const rows = JSON.parse(content || '[]')
    if (!Array.isArray(rows) || !rows.length) return emptyFormRow()
    return rows.map(r => ({
      key: (r && r.key) || '',
      value: (r && r.value) || '',
      enabled: !r || r.enabled !== false,
      type: r && r.type === 'file' ? 'file' : 'text',
      file_path: (r && r.file_path) || '',
      file_name: (r && r.file_name) || ''
    }))
  } catch (e) {
    return emptyFormRow()
  }
}

function joinBaseUrl(base, url) {
  const b = String(base || '').trim().replace(/\/+$/, '')
  const u = String(url || '').trim()
  if (!u) return b
  if (/^https?:\/\//i.test(u) || u.startsWith('{{')) return u
  if (!b) return u
  return u.startsWith('/') ? b + u : b + '/' + u
}

function isAbsoluteOrVarUrl(url) {
  const u = String(url || '').trim()
  return /^https?:\/\//i.test(u) || u.startsWith('{{')
}

function substituteVars(input, map) {
  let out = String(input || '')
  // {{key}} with optional spaces
  out = out.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (full, rawKey) => {
    const key = String(rawKey || '').trim()
    if (!key) return full
    if (Object.prototype.hasOwnProperty.call(map, key)) {
      return map[key] == null ? '' : String(map[key])
    }
    const found = Object.keys(map).find(k => k.toLowerCase() === key.toLowerCase())
    if (found) return map[found] == null ? '' : String(map[found])
    return full
  })
  return out
}

const SPLIT_STORAGE_KEY = 'apitest-editor-split-percent'
const SPLIT_DEFAULT = 42
const SPLIT_MIN = 18
const SPLIT_MAX = 78

function loadSplitPercent() {
  try {
    const n = Number(localStorage.getItem(SPLIT_STORAGE_KEY))
    if (Number.isFinite(n) && n >= SPLIT_MIN && n <= SPLIT_MAX) return n
  } catch (e) { /* ignore */ }
  return SPLIT_DEFAULT
}

function saveSplitPercent(n) {
  try {
    localStorage.setItem(SPLIT_STORAGE_KEY, String(Math.round(n)))
  } catch (e) { /* ignore */ }
}

export default {
  name: 'ApiWorkspace',
  components: { KvEditor, FormDataEditor, JsonViewer, AtIcon },
  data() {
    return {
      isTauri: false,
      methods: METHODS,
      commonHeaderKeys: COMMON_HEADER_KEYS,
      commonHeaderValues: COMMON_HEADER_VALUES,
      leftTab: 'collections',
      treeData: [],
      historyList: [],
      environments: [],
      activeEnvId: null,
      currentRequest: null,
      selectedCollectionId: null,
      formBodyRows: emptyKv(),
      graphqlQuery: '',
      graphqlVariables: '{}',
      binaryFilePath: '',
      binaryFileName: '',
      rawLanguages: RAW_LANGUAGES,
      reqTab: 'params',
      respTab: 'body',
      response: null,
      sending: false,
      saving: false,
      splitPercent: loadSplitPercent(),
      splitting: false,
      envDialogVisible: false,
      envVarRows: emptyKv(),
      savingEnv: false,
      originalEnvVars: [],
      importDialogVisible: false,
      importJson: '',
      importing: false,
      workspaces: [],
      activeWorkspaceId: null,
      theme: 'light',
      themeDark: false,
      testResults: [],
      openMenu: null,
      helpDialogVisible: false,
      settingsDialogVisible: false,
      settingsTab: 'general',
      dbPath: '',
      dbVersion: '',
      winMaximized: false,
      collectionDialogVisible: false,
      collectionSaving: false,
      collectionForm: {
        id: null,
        name: '',
        base_url: ''
      },
      jmeterExportDialogVisible: false,
      jmeterExporting: false,
      jmeterForm: {
        threads: 10,
        loops: 10,
        ramp_up: 1
      },
      loadTestDialogVisible: false,
      loadTesting: false,
      loadTestForm: {
        threads: 10,
        loops: 10,
        ramp_up_secs: 1
      },
      loadTestResult: null
    }
  },
  computed: {
    menus() {
      return [
        {
          key: 'file',
          label: '文件',
          items: [
            { label: '新建请求', action: 'new-request', shortcut: 'Ctrl+N' },
            { label: '新建集合', action: 'new-collection' },
            { label: '新建工作区', action: 'new-workspace' },
            { type: 'divider' },
            { label: '导入集合…', action: 'import' },
            { label: '导出集合…', action: 'export', disabled: () => !this.selectedCollectionId },
            { label: '导出 JMeter…', action: 'export-jmeter', disabled: () => !this.selectedCollectionId },
            { type: 'divider' },
            { label: '环境变量…', action: 'env' },
            { type: 'divider' },
            { label: '设置…', action: 'settings', shortcut: 'Ctrl+,' }
          ]
        },
        {
          key: 'edit',
          label: '编辑',
          items: [
            { label: '保存请求', action: 'save', shortcut: 'Ctrl+S', disabled: () => !this.currentRequest },
            { label: '发送请求', action: 'send', shortcut: 'Ctrl+Enter', disabled: () => !this.currentRequest },
            { label: '压测当前请求…', action: 'loadtest', disabled: () => !this.currentRequest }
          ]
        },
        {
          key: 'view',
          label: '视图',
          items: [
            { label: '集合', action: 'view-collections' },
            { label: '历史', action: 'view-history' },
            { type: 'divider' },
            { label: this.themeDark ? '切换到亮色主题' : '切换到暗色主题', action: 'toggle-theme' },
            { label: '刷新侧栏', action: 'refresh' },
            { type: 'divider' },
            { label: '设置…', action: 'settings' }
          ]
        },
        {
          key: 'help',
          label: '帮助',
          items: [
            { label: '快捷键', action: 'shortcuts' },
            { label: '设置…', action: 'settings' },
            { type: 'divider' },
            { label: '关于 ApiTest', action: 'about' }
          ]
        }
      ]
    },
    urlPlaceholder() {
      if (this.collectionBaseUrl) {
        return '相对路径，如 /users'
      }
      return '输入请求 URL，如 https://api.example.com/path'
    },
    collectionBaseUrl() {
      const id = this.currentRequest && this.currentRequest.collection_id
        ? this.currentRequest.collection_id
        : this.selectedCollectionId
      if (!id) return ''
      const node = this.findCollectionNode(this.treeData, id)
      return (node && node.base_url) || ''
    },
    urlIgnoresCollectionPrefix() {
      if (!this.currentRequest) return false
      return isAbsoluteOrVarUrl(this.currentRequest.url)
    },
    resolvedRequestUrl() {
      if (!this.currentRequest) return ''
      const map = this.buildEnvMap()
      const joined = joinBaseUrl(this.collectionBaseUrl, this.currentRequest.url || '')
      return substituteVars(joined, map)
    },
    urlInputDisplay() {
      if (!this.currentRequest) return ''
      // Always show substituted value ({{baseUrl}} → actual host)
      const resolved = this.resolvedRequestUrl
      return resolved || this.currentRequest.url || ''
    },
    varExample() {
      return '{{baseUrl}}'
    },
    isFormBodyType() {
      return this.currentRequest && FORM_BODY_TYPES.includes(this.currentRequest.body_type)
    },
    bodyPlaceholder() {
      if (!this.currentRequest) return '请求体'
      if (this.currentRequest.body_type === 'raw') {
        const lang = this.currentRequest.body_language || 'text'
        if (lang === 'json') return '{ "key": "value" }'
        if (lang === 'xml') return '<root></root>'
        if (lang === 'html') return '<html></html>'
        if (lang === 'javascript') return '// javascript'
        return 'raw body'
      }
      return '请求体'
    },
    canBeautifyBody() {
      if (!this.currentRequest) return false
      if (this.currentRequest.body_type === 'graphql') return true
      return this.currentRequest.body_type === 'raw' &&
        ['json', 'xml'].includes(this.currentRequest.body_language)
    },
    testTabLabel() {
      if (!this.testResults.length) return 'Test Results'
      const pass = this.testResults.filter(t => t.passed).length
      return `Test Results (${pass}/${this.testResults.length})`
    },
    headersTabLabel() {
      const n = this.response && this.response.headers ? this.response.headers.length : 0
      return n ? `Headers (${n})` : 'Headers'
    },
    responseBodyBytes() {
      if (!this.response || !this.response.body) return 0
      try {
        return typeof TextEncoder !== 'undefined'
          ? new TextEncoder().encode(this.response.body).length
          : this.response.body.length
      } catch (e) {
        return this.response.body.length
      }
    },
    responseContentType() {
      if (!this.response || !this.response.headers) return ''
      const hit = this.response.headers.find(h =>
        h.key && h.key.toLowerCase() === 'content-type'
      )
      return hit ? (hit.value || '') : ''
    },
    reqSectionStyle() {
      return {
        height: this.splitPercent + '%',
        flex: 'none',
        maxHeight: 'none'
      }
    },
    importPlaceholder() {
      return '粘贴 Postman collection.json / swagger.json / openapi.yaml，或 ApiTest 导出的 JSON…'
    }
  },
  async mounted() {
    this.isTauri = isTauriRuntime()
    document.addEventListener('click', this.closeMenus)
    if (this.isTauri) {
      this.syncWinMaximized()
    }
    if (!this.isTauri) {
      this.seedBrowserPreview()
      this.applyDomTheme()
      return
    }
    await this.bootstrap()
  },
  beforeDestroy() {
    document.removeEventListener('click', this.closeMenus)
    document.body.classList.remove('apitest-theme-dark')
  },
  methods: {
    async syncWinMaximized() {
      try {
        this.winMaximized = await windowIsMaximized()
      } catch (e) {
        this.winMaximized = false
      }
    },
    async onTitleDrag(e) {
      if (!this.isTauri) return
      // only left button; ignore interactive children already stopped
      if (e.button !== 0) return
      try {
        await windowStartDragging()
      } catch (err) {
        // ignore
      }
    },
    async onWinMinimize() {
      try { await windowMinimize() } catch (e) { /* ignore */ }
    },
    async onWinMaximize() {
      try {
        await windowToggleMaximize()
        await this.syncWinMaximized()
      } catch (e) { /* ignore */ }
    },
    async onWinClose() {
      try { await windowClose() } catch (e) { /* ignore */ }
    },
    toggleMenu(key) {
      this.openMenu = this.openMenu === key ? null : key
    },
    closeMenus() {
      this.openMenu = null
    },
    onMenuEnter(key) {
      if (this.openMenu) this.openMenu = key
    },
    onMenuAction(item) {
      if (item.disabled && item.disabled()) return
      this.closeMenus()
      const map = {
        'new-request': () => this.onAddRequest(),
        'new-collection': () => this.onAddCollection(),
        'new-workspace': () => this.onAddWorkspace(),
        import: () => { this.importDialogVisible = true },
        export: () => this.onExportCollection(),
        'export-jmeter': () => { this.jmeterExportDialogVisible = true },
        env: () => { this.envDialogVisible = true },
        settings: () => this.openSettings(),
        save: () => this.onSave(),
        send: () => this.onSend(),
        loadtest: () => this.openLoadTest(),
        'view-collections': () => { this.leftTab = 'collections' },
        'view-history': () => { this.leftTab = 'history' },
        'toggle-theme': () => this.toggleTheme(),
        refresh: () => { this.loadTree(); this.loadHistory() },
        shortcuts: () => { this.helpDialogVisible = true },
        about: () => this.$message.info('ApiTest — 本地 API 调试工具 (Tauri + Vue)')
      }
      const fn = map[item.action]
      if (fn) fn()
    },
    seedBrowserPreview() {
      this.workspaces = [{ id: 1, name: 'My Workspace', is_active: true }]
      this.activeWorkspaceId = 1
      this.environments = [{ id: 1, name: 'Local', is_active: true }]
      this.activeEnvId = 1
      this.treeData = [{
        id: 'col-1',
        node_type: 'collection',
        label: 'Demo Collection',
        collection_id: 1,
        base_url: 'https://api.example.com/v1',
        children: [{
          id: 'req-1',
          node_type: 'request',
          label: 'Get Users',
          method: 'GET',
          request_id: 1,
          collection_id: 1
        }, {
          id: 'req-2',
          node_type: 'request',
          label: 'Create User',
          method: 'POST',
          request_id: 2,
          collection_id: 1
        }]
      }]
      this.selectedCollectionId = 1
      this.applyRequest({
        id: 1,
        collection_id: 1,
        name: 'Get Users',
        method: 'GET',
        url: '/users',
        params: [{ key: 'page', value: '1', enabled: true }],
        headers: [{ key: 'Accept', value: 'application/json', enabled: true }],
        body_type: 'none',
        body_content: '',
        pre_script: '',
        test_script: '',
        mock_enabled: false,
        mock_status: 200,
        mock_headers: emptyKv(),
        mock_body: '',
        mock_delay_ms: 0
      })
    },
    async openSettings() {
      this.settingsDialogVisible = true
      this.settingsTab = 'general'
      if (!this.isTauri) {
        this.dbPath = '(浏览器预览不可用)'
        this.dbVersion = '-'
        return
      }
      try {
        const [pathRes, verRes] = await Promise.all([api.getDbPath(), api.getDbVersion()])
        this.dbPath = (pathRes && pathRes.path) || ''
        this.dbVersion = (verRes && verRes.schema_version) || ''
      } catch (e) {
        this.dbPath = this.errMsg(e)
        this.dbVersion = '-'
      }
    },
    async onSettingsThemeChange(val) {
      this.themeDark = val === 'dark'
      await this.onThemeChange(this.themeDark)
    },
    onKeydown(e) {
      const key = (e.key || '').toLowerCase()
      const meta = e.ctrlKey || e.metaKey
      if (!meta) return
      if (key === 'enter') {
        e.preventDefault()
        this.onSend()
      } else if (key === 's') {
        e.preventDefault()
        this.onSave()
      } else if (key === 'n') {
        e.preventDefault()
        this.onAddRequest()
      } else if (key === ',') {
        e.preventDefault()
        this.openSettings()
      }
    },
    shortMethod(method) {
      const m = (method || 'GET').toUpperCase()
      if (m === 'DELETE') return 'DEL'
      if (m === 'OPTIONS') return 'OPT'
      if (m === 'PATCH') return 'PAT'
      return m
    },
    statusClass(status) {
      if (!status) return 's-muted'
      if (status >= 200 && status < 300) return 's-ok'
      if (status >= 400) return 's-err'
      return 's-warn'
    },
    statusType(status) {
      if (!status) return 'info'
      if (status >= 200 && status < 300) return 'success'
      if (status >= 400) return 'danger'
      return 'warning'
    },
    toggleTheme() {
      this.themeDark = !this.themeDark
      this.onThemeChange(this.themeDark)
    },
    async bootstrap() {
      try {
        await this.loadTheme()
        await this.loadWorkspaces()
        await this.loadEnvironments()
        await this.loadTree()
        await this.loadHistory()
        const first = this.findFirstRequest(this.treeData)
        if (first) {
          await this.openRequest(first.request_id)
          this.$nextTick(() => {
            if (this.$refs.tree) this.$refs.tree.setCurrentKey(`req-${first.request_id}`)
          })
        }
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    envDisplayName(name) {
      return String(name || '').replace(/#ws\d+$/, '') || name
    },
    async loadTheme() {
      this.theme = await api.getTheme()
      this.themeDark = this.theme === 'dark'
      this.applyDomTheme()
    },
    async onThemeChange(val) {
      this.theme = val ? 'dark' : 'light'
      this.applyDomTheme()
      try {
        await api.setTheme(this.theme)
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    applyDomTheme() {
      document.body.classList.toggle('apitest-theme-dark', this.theme === 'dark')
    },
    async loadWorkspaces() {
      this.workspaces = await api.listWorkspaces()
      const active = this.workspaces.find(w => w.is_active) || this.workspaces[0]
      this.activeWorkspaceId = active ? active.id : null
    },
    async onWorkspaceChange(id) {
      try {
        await api.setActiveWorkspace(id)
        this.currentRequest = null
        this.response = null
        this.testResults = []
        await this.loadEnvironments()
        await this.loadTree()
        this.$message.success('已切换工作区')
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    async onAddWorkspace() {
      try {
        const { value } = await this.$prompt('工作区名称', '新建工作区', {
          confirmButtonText: '创建',
          cancelButtonText: '取消',
          inputValue: 'New Workspace',
          inputPattern: /\S+/
        })
        const ws = await api.createWorkspace(value)
        await api.setActiveWorkspace(ws.id)
        await this.loadWorkspaces()
        await this.onWorkspaceChange(ws.id)
      } catch (e) {
        if (e === 'cancel') return
        this.$message.error(this.errMsg(e))
      }
    },
    findFirstRequest(nodes) {
      for (const n of nodes || []) {
        if (n.node_type === 'request') return n
        const child = this.findFirstRequest(n.children)
        if (child) return child
      }
      return null
    },
    errMsg(e) {
      return (e && e.message) || String(e)
    },
    formatBytes(n) {
      const num = Number(n) || 0
      if (num < 1024) return `${num} B`
      if (num < 1024 * 1024) return `${(num / 1024).toFixed(1)} KB`
      return `${(num / (1024 * 1024)).toFixed(2)} MB`
    },
    formatDuration(ms) {
      const n = Number(ms) || 0
      if (n < 1000) return `${n} ms`
      return `${(n / 1000).toFixed(2)} s`
    },
    shortContentType(ct) {
      if (!ct) return ''
      const main = String(ct).split(';')[0].trim()
      if (main.length <= 28) return main
      return main.slice(0, 26) + '…'
    },
    async copyText(text, successMsg) {
      if (text == null || text === '') return
      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(String(text))
        } else {
          const ta = document.createElement('textarea')
          ta.value = String(text)
          ta.style.position = 'fixed'
          ta.style.left = '-9999px'
          document.body.appendChild(ta)
          ta.select()
          document.execCommand('copy')
          document.body.removeChild(ta)
        }
        this.$message.success(successMsg || '已复制')
      } catch (e) {
        this.$message.error('复制失败')
      }
    },
    clampSplitPercent(n) {
      return Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, n))
    },
    resetSplitPercent() {
      this.splitPercent = SPLIT_DEFAULT
      saveSplitPercent(this.splitPercent)
    },
    onSplitMouseDown(e) {
      if (e.button !== 0) return
      e.preventDefault()
      const el = this.$refs.editorSplit
      if (!el) return
      this.splitting = true
      const onMove = (ev) => {
        const rect = el.getBoundingClientRect()
        if (!rect.height) return
        const pct = ((ev.clientY - rect.top) / rect.height) * 100
        this.splitPercent = this.clampSplitPercent(pct)
      }
      const onUp = () => {
        this.splitting = false
        document.removeEventListener('mousemove', onMove)
        document.removeEventListener('mouseup', onUp)
        document.body.style.cursor = ''
        document.body.style.userSelect = ''
        saveSplitPercent(this.splitPercent)
      }
      document.body.style.cursor = 'row-resize'
      document.body.style.userSelect = 'none'
      document.addEventListener('mousemove', onMove)
      document.addEventListener('mouseup', onUp)
    },
    async loadTree() {
      this.treeData = await api.getSidebarTree()
    },
    async loadHistory() {
      this.historyList = await api.listHistory(100)
    },
    async loadEnvironments() {
      this.environments = await api.listEnvironments()
      const active = this.environments.find(e => e.is_active) || this.environments[0]
      this.activeEnvId = active ? active.id : null
      if (this.activeEnvId) await this.loadEnvVars()
    },
    async loadEnvVars() {
      if (!this.activeEnvId) return
      this.originalEnvVars = await api.listEnvVars(this.activeEnvId)
      this.envVarRows = this.originalEnvVars.map(v => ({
        key: v.key,
        value: v.value,
        enabled: v.enabled,
        id: v.id
      }))
      if (!this.envVarRows.length) this.envVarRows = emptyKv()
    },
    async onEnvChange(id) {
      try {
        await api.setActiveEnvironment(id)
        await this.loadEnvVars()
        this.$message.success('已切换环境')
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    async onNodeClick(data) {
      if (data.node_type === 'collection') {
        this.selectedCollectionId = data.collection_id
        return
      }
      if (data.node_type === 'request' && data.request_id) {
        await this.openRequest(data.request_id)
      }
    },
    applyRequest(req) {
      const normalized = normalizeBodyFields({ ...req })
      this.currentRequest = {
        ...normalized,
        params: normalized.params && normalized.params.length ? normalized.params : emptyKv(),
        headers: normalized.headers && normalized.headers.length ? normalized.headers : emptyKv(),
        body_type: normalized.body_type || 'none',
        body_content: normalized.body_content || '',
        body_language: normalized.body_language || (normalized.body_type === 'raw' ? 'json' : ''),
        pre_script: normalized.pre_script || '',
        test_script: normalized.test_script || '',
        mock_enabled: !!normalized.mock_enabled,
        mock_status: normalized.mock_status || 200,
        mock_headers: normalized.mock_headers && normalized.mock_headers.length ? normalized.mock_headers : emptyKv(),
        mock_body: normalized.mock_body || '',
        mock_delay_ms: normalized.mock_delay_ms || 0
      }
      this.selectedCollectionId = normalized.collection_id || this.selectedCollectionId
      this.testResults = []
      this.hydrateBodyEditors()
    },
    hydrateBodyEditors() {
      const req = this.currentRequest
      if (!req) return
      if (FORM_BODY_TYPES.includes(req.body_type)) {
        this.formBodyRows = parseFormRows(req.body_content)
      } else {
        this.formBodyRows = emptyKv()
      }
      if (req.body_type === 'binary') {
        const meta = parseBinaryMeta(req.body_content)
        this.binaryFilePath = meta.file_path
        this.binaryFileName = meta.file_name
      } else {
        this.binaryFilePath = ''
        this.binaryFileName = ''
      }
      if (req.body_type === 'graphql') {
        const g = parseGraphqlContent(req.body_content)
        this.graphqlQuery = g.query
        this.graphqlVariables = g.variables
      } else {
        this.graphqlQuery = ''
        this.graphqlVariables = '{}'
      }
    },
    async openRequest(id) {
      try {
        const req = await api.getRequest(id)
        this.applyRequest(req)
        this.response = null
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    onBodyTypeChange(type) {
      if (!this.currentRequest) return
      if (FORM_BODY_TYPES.includes(type)) {
        this.formBodyRows = parseFormRows(this.currentRequest.body_content)
        this.syncFormBodyToContent()
      } else if (type === 'raw') {
        if (!this.currentRequest.body_language) {
          this.currentRequest.body_language = 'json'
        }
      } else if (type === 'binary') {
        const meta = parseBinaryMeta(this.currentRequest.body_content)
        this.binaryFilePath = meta.file_path
        this.binaryFileName = meta.file_name
        if (!meta.file_path) this.currentRequest.body_content = ''
      } else if (type === 'graphql') {
        const g = parseGraphqlContent(this.currentRequest.body_content)
        this.graphqlQuery = g.query || ''
        this.graphqlVariables = g.variables || '{}'
        this.syncGraphqlToContent()
      } else if (type === 'none') {
        this.currentRequest.body_content = ''
        this.currentRequest.body_language = ''
      }
    },
    onRawLanguageChange() {
      // Content-Type is applied at send time from body_language
    },
    syncFormBodyToContent() {
      if (!this.currentRequest) return
      this.currentRequest.body_content = JSON.stringify(this.formBodyRows || [])
    },
    syncGraphqlToContent() {
      if (!this.currentRequest) return
      this.currentRequest.body_content = JSON.stringify({
        query: this.graphqlQuery || '',
        variables: this.graphqlVariables || '{}'
      }, null, 2)
    },
    syncBinaryToContent() {
      if (!this.currentRequest) return
      if (!this.binaryFilePath) {
        this.currentRequest.body_content = ''
        return
      }
      this.currentRequest.body_content = JSON.stringify({
        file_path: this.binaryFilePath,
        file_name: this.binaryFileName || ''
      })
    },
    pickBinaryFile() {
      const input = this.$refs.binaryFileInput
      if (input) {
        input.value = ''
        input.click()
      }
    },
    onBinaryFilePicked(e) {
      const file = e.target && e.target.files && e.target.files[0]
      if (!file) return
      // In Tauri/WebView, path may be available as file.path
      const path = file.path || file.name
      this.binaryFilePath = path
      this.binaryFileName = file.name || ''
      this.syncBinaryToContent()
      if (!file.path) {
        this.$message.warning('浏览器预览无法获取本地绝对路径，请在桌面应用中选择文件')
      }
    },
    clearBinaryFile() {
      this.binaryFilePath = ''
      this.binaryFileName = ''
      this.syncBinaryToContent()
    },
    beautifyBody() {
      if (!this.currentRequest) return
      try {
        if (this.currentRequest.body_type === 'graphql') {
          const parsed = JSON.parse(this.graphqlVariables || '{}')
          this.graphqlVariables = JSON.stringify(parsed, null, 2)
          this.syncGraphqlToContent()
          this.$message.success('Variables 已格式化')
          return
        }
        const lang = this.currentRequest.body_language
        const raw = this.currentRequest.body_content || ''
        if (lang === 'json') {
          this.currentRequest.body_content = JSON.stringify(JSON.parse(raw), null, 2)
          this.$message.success('已格式化')
        } else if (lang === 'xml') {
          this.currentRequest.body_content = beautifyXml(raw)
          this.$message.success('已格式化')
        }
      } catch (e) {
        this.$message.error('格式化失败: ' + this.errMsg(e))
      }
    },
    buildSavePayload() {
      if (this.isFormBodyType) this.syncFormBodyToContent()
      if (this.currentRequest.body_type === 'graphql') this.syncGraphqlToContent()
      if (this.currentRequest.body_type === 'binary') this.syncBinaryToContent()
      return {
        id: this.currentRequest.id,
        name: this.currentRequest.name,
        method: this.currentRequest.method,
        url: this.currentRequest.url,
        params: this.currentRequest.params,
        headers: this.currentRequest.headers,
        body_type: this.currentRequest.body_type,
        body_content: this.currentRequest.body_content,
        body_language: this.currentRequest.body_language || '',
        collection_id: this.currentRequest.collection_id,
        pre_script: this.currentRequest.pre_script || '',
        test_script: this.currentRequest.test_script || '',
        mock_enabled: !!this.currentRequest.mock_enabled,
        mock_status: this.currentRequest.mock_status || 200,
        mock_headers: this.currentRequest.mock_headers || [],
        mock_body: this.currentRequest.mock_body || '',
        mock_delay_ms: this.currentRequest.mock_delay_ms || 0
      }
    },
    async applyEnvUpdates(envUpdates) {
      if (!this.activeEnvId || !envUpdates) return
      for (const key of Object.keys(envUpdates)) {
        const value = envUpdates[key]
        if (value === null) {
          const old = (this.originalEnvVars || []).find(v => v.key === key)
          if (old) await api.deleteEnvVar(old.id)
        } else {
          await api.upsertEnvVar({
            environment_id: this.activeEnvId,
            key,
            value: String(value),
            enabled: true
          })
        }
      }
      await this.loadEnvVars()
    },
    buildEnvMap() {
      const map = {}
      ;(this.envVarRows || []).forEach(row => {
        if (row.enabled !== false && row.key) map[row.key] = row.value || ''
      })
      // Fallback: collection prefix fills baseUrl when env var missing
      const colBase = (this.collectionBaseUrl || '').trim().replace(/\/+$/, '')
      if (colBase) {
        if (!map.baseUrl) map.baseUrl = colBase
        if (!map.baseURL) map.baseURL = colBase
      }
      return map
    },
    onUrlInputChange(val) {
      if (!this.currentRequest) return
      const next = val == null ? '' : String(val)
      const resolved = this.resolvedRequestUrl
      // Keep original template when display-only value didn't really change
      if (resolved && next === resolved && /\{\{/.test(this.currentRequest.url || '')) {
        return
      }
      this.currentRequest.url = next
    },
    async onAddCollection() {
      this.collectionForm = { id: null, name: 'New Collection', base_url: '' }
      this.collectionDialogVisible = true
    },
    onEditCollection(data) {
      this.collectionForm = {
        id: data.collection_id,
        name: data.label || '',
        base_url: data.base_url || ''
      }
      this.collectionDialogVisible = true
    },
    async saveCollection() {
      const name = (this.collectionForm.name || '').trim()
      if (!name) {
        this.$message.warning('名称不能为空')
        return
      }
      this.collectionSaving = true
      try {
        const base_url = (this.collectionForm.base_url || '').trim()
        if (this.collectionForm.id) {
          await api.updateCollection({
            id: this.collectionForm.id,
            name,
            base_url
          })
        } else {
          const created = await api.createCollection({
            name,
            parent_id: null,
            base_url
          })
          this.selectedCollectionId = created.id
        }
        this.collectionDialogVisible = false
        await this.loadTree()
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.collectionSaving = false
      }
    },
    findCollectionNode(nodes, collectionId) {
      if (!nodes || !nodes.length) return null
      for (const n of nodes) {
        if (n.node_type === 'collection' && n.collection_id === collectionId) return n
        const found = this.findCollectionNode(n.children, collectionId)
        if (found) return found
      }
      return null
    },
    async onAddRequest() {
      try {
        const created = await api.createRequest({
          collection_id: this.selectedCollectionId,
          name: 'New Request',
          method: 'GET',
          url: this.collectionBaseUrl ? '/' : '{{baseUrl}}/'
        })
        await this.loadTree()
        await this.openRequest(created.id)
        this.$nextTick(() => {
          if (this.$refs.tree) this.$refs.tree.setCurrentKey(`req-${created.id}`)
        })
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    async onDeleteNode(data) {
      try {
        await this.$confirm(
          data.node_type === 'collection' ? '删除集合及其下请求？' : '删除该请求？',
          '确认',
          { type: 'warning' }
        )
        if (data.node_type === 'collection') {
          await api.deleteCollection(data.collection_id)
          if (this.currentRequest && this.currentRequest.collection_id === data.collection_id) {
            this.currentRequest = null
          }
          if (this.selectedCollectionId === data.collection_id) this.selectedCollectionId = null
        } else {
          await api.deleteRequest(data.request_id)
          if (this.currentRequest && this.currentRequest.id === data.request_id) {
            this.currentRequest = null
          }
        }
        this.response = null
        await this.loadTree()
      } catch (e) {
        if (e === 'cancel') return
        this.$message.error(this.errMsg(e))
      }
    },
    async onSave() {
      if (!this.currentRequest || !this.currentRequest.id) return
      this.saving = true
      try {
        await api.saveRequest(this.buildSavePayload())
        await this.loadTree()
        this.$message.success('已保存')
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.saving = false
      }
    },
    async onSend() {
      if (!this.currentRequest || this.sending) return
      const urlEmpty = !this.currentRequest.url || !this.currentRequest.url.trim()
      if (urlEmpty && !this.collectionBaseUrl) {
        this.$message.warning('请填写 URL')
        return
      }
      if (this.isFormBodyType) this.syncFormBodyToContent()
      if (this.currentRequest.body_type === 'graphql') this.syncGraphqlToContent()
      if (this.currentRequest.body_type === 'binary') this.syncBinaryToContent()

      this.sending = true
      this.response = null
      this.testResults = []
      try {
        let sendPayload = {
          method: this.currentRequest.method,
          url: this.currentRequest.url,
          params: this.currentRequest.params,
          headers: this.currentRequest.headers,
          body_type: this.currentRequest.body_type,
          body_content: this.currentRequest.body_content,
          body_language: this.currentRequest.body_language || '',
          request_id: this.currentRequest.id,
          environment_id: this.activeEnvId
        }

        try {
          const pre = runPreScript(this.currentRequest.pre_script, {
            envMap: this.buildEnvMap(),
            request: sendPayload
          })
          sendPayload = {
            ...sendPayload,
            method: pre.request.method,
            url: pre.request.url,
            params: pre.request.params,
            headers: pre.request.headers,
            body_type: pre.request.body_type,
            body_content: pre.request.body_content
          }
          this.currentRequest.method = sendPayload.method
          this.currentRequest.url = sendPayload.url
          this.currentRequest.params = sendPayload.params
          this.currentRequest.headers = sendPayload.headers
          this.currentRequest.body_type = sendPayload.body_type
          this.currentRequest.body_content = sendPayload.body_content
          await this.applyEnvUpdates(pre.envUpdates)
        } catch (e) {
          this.$message.error('前置脚本错误: ' + this.errMsg(e))
          return
        }

        if (this.currentRequest.id) {
          try {
            await api.saveRequest(this.buildSavePayload())
          } catch (e) {
            // continue
          }
        }

        this.response = await api.sendRequest(sendPayload)

        if (this.response && this.response.status_text === 'CANCELLED') {
          this.respTab = 'body'
          this.testResults = []
          return
        }

        try {
          const result = runTestScript(this.currentRequest.test_script, {
            envMap: this.buildEnvMap(),
            request: sendPayload,
            response: this.response
          })
          this.testResults = result.tests || []
          await this.applyEnvUpdates(result.envUpdates)
          if (this.testResults.length) this.respTab = 'tests'
          else this.respTab = 'body'
        } catch (e) {
          this.testResults = [{ name: 'script error', passed: false, error: this.errMsg(e) }]
          this.respTab = 'tests'
        }

        await this.loadHistory()
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.sending = false
      }
    },
    async onCancelSend() {
      if (!this.sending) return
      try {
        await api.cancelRequest()
      } catch (e) {
        // ignore — send will settle shortly
      }
    },
    async onOpenHistory(item) {
      try {
        const detail = await api.getHistory(item.id)
        let reqSnap = null
        let respSnap = null
        try { reqSnap = JSON.parse(detail.request_snapshot || 'null') } catch (e) { /* ignore */ }
        try { respSnap = JSON.parse(detail.response_snapshot || 'null') } catch (e) { /* ignore */ }

        if (reqSnap) {
          this.applyRequest({
            id: detail.request_id,
            collection_id: this.selectedCollectionId,
            name: `History #${detail.id}`,
            method: reqSnap.method || detail.method,
            url: reqSnap.url || detail.url,
            params: reqSnap.params || emptyKv(),
            headers: reqSnap.headers || emptyKv(),
            body_type: reqSnap.body_type || 'none',
            body_content: reqSnap.body_content || '',
            body_language: reqSnap.body_language || ''
          })
        }
        if (respSnap) {
          this.response = respSnap
          this.respTab = 'body'
        }
        this.leftTab = 'collections'
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    async onClearHistory() {
      try {
        await this.$confirm('清空全部请求历史？', '确认', { type: 'warning' })
        await api.clearHistory()
        this.historyList = []
        this.$message.success('历史已清空')
      } catch (e) {
        if (e === 'cancel') return
        this.$message.error(this.errMsg(e))
      }
    },
    async onExportCollection() {
      if (!this.selectedCollectionId) {
        this.$message.warning('请先选中一个集合')
        return
      }
      try {
        const data = await api.exportCollection(this.selectedCollectionId)
        const text = JSON.stringify(data, null, 2)
        const blob = new Blob([text], { type: 'application/json' })
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `${data.name || 'collection'}.apitest.json`
        a.click()
        URL.revokeObjectURL(url)
        this.$message.success('已导出')
      } catch (e) {
        this.$message.error(this.errMsg(e))
      }
    },
    async onExportJmeter() {
      if (!this.selectedCollectionId) {
        this.$message.warning('请先选中一个集合')
        return
      }
      this.jmeterExporting = true
      try {
        const data = await api.exportJmeter({
          collection_id: this.selectedCollectionId,
          threads: this.jmeterForm.threads,
          loops: this.jmeterForm.loops,
          ramp_up: this.jmeterForm.ramp_up
        })
        const blob = new Blob([data.jmx], { type: 'application/xml' })
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `${data.name || 'collection'}.jmx`
        a.click()
        URL.revokeObjectURL(url)
        this.jmeterExportDialogVisible = false
        this.$message.success('已导出 JMeter 测试计划')
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.jmeterExporting = false
      }
    },
    openLoadTest() {
      if (!this.currentRequest) {
        this.$message.warning('请先打开一个请求')
        return
      }
      this.loadTestResult = null
      this.loadTestDialogVisible = true
    },
    formatRps(v) {
      const n = Number(v) || 0
      return n >= 100 ? n.toFixed(0) : n.toFixed(2)
    },
    formatMs(v) {
      const n = Number(v) || 0
      return `${n.toFixed(1)} ms`
    },
    async onRunLoadTest() {
      if (!this.currentRequest) return
      const urlEmpty = !this.currentRequest.url || !this.currentRequest.url.trim()
      if (urlEmpty && !this.collectionBaseUrl) {
        this.$message.warning('请填写 URL')
        return
      }
      if (this.isFormBodyType) this.syncFormBodyToContent()
      if (this.currentRequest.body_type === 'graphql') this.syncGraphqlToContent()
      if (this.currentRequest.body_type === 'binary') this.syncBinaryToContent()
      this.loadTesting = true
      this.loadTestResult = null
      try {
        this.loadTestResult = await api.runLoadTest({
          method: this.currentRequest.method,
          url: this.currentRequest.url || '',
          params: this.currentRequest.params,
          headers: this.currentRequest.headers,
          body_type: this.currentRequest.body_type,
          body_content: this.currentRequest.body_content,
          body_language: this.currentRequest.body_language || '',
          request_id: this.currentRequest.id,
          environment_id: this.activeEnvId,
          threads: this.loadTestForm.threads,
          loops: this.loadTestForm.loops,
          ramp_up_secs: this.loadTestForm.ramp_up_secs
        })
        this.$message.success(
          `压测完成：${this.loadTestResult.success}/${this.loadTestResult.total} 成功`
        )
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.loadTesting = false
      }
    },
    async onImportCollection() {
      if (!this.importJson.trim()) {
        this.$message.warning('请粘贴 JSON')
        return
      }
      this.importing = true
      try {
        const created = await api.importCollection(this.importJson)
        this.importDialogVisible = false
        this.importJson = ''
        this.selectedCollectionId = created.id
        await this.loadTree()
        this.$message.success(`已导入集合：${created.name}`)
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.importing = false
      }
    },
    async saveEnvVars() {
      if (!this.activeEnvId) return
      this.savingEnv = true
      try {
        const keepKeys = new Set()
        for (const row of this.envVarRows) {
          if (!row.key || !row.key.trim()) continue
          keepKeys.add(row.key.trim())
          await api.upsertEnvVar({
            environment_id: this.activeEnvId,
            key: row.key.trim(),
            value: row.value || '',
            enabled: row.enabled !== false
          })
        }
        for (const old of this.originalEnvVars) {
          if (!keepKeys.has(old.key)) await api.deleteEnvVar(old.id)
        }
        await this.loadEnvVars()
        this.envDialogVisible = false
        this.$message.success('环境变量已保存')
      } catch (e) {
        this.$message.error(this.errMsg(e))
      } finally {
        this.savingEnv = false
      }
    }
  }
}
</script>

<style lang="scss" scoped>
$orange: #ff6c37;
$bg: #f5f5f5;
$sidebar: #1c1f26;
$sidebar-text: #c8cdd5;
$border: #e5e7eb;
$panel: #ffffff;

.apitest-workspace {
  --at-input-bg: #ffffff;
  --at-input-border: #dcdfe6;
  --at-input-color: #1f2937;
  --at-input-placeholder: #9ca3af;
  --at-input-hover-border: #c0c4cc;
  --at-surface: #ffffff;
  --at-surface-muted: #f8fafc;
  --at-border-soft: #eef0f3;
  --at-text-muted: #6b7280;

  height: 100vh;
  display: flex;
  flex-direction: column;
  background: $bg;
  outline: none;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
  color: #1f2937;

  &.theme-dark {
    --at-input-bg: #1c212b;
    --at-input-border: #2a2e38;
    --at-input-color: #e5e7eb;
    --at-input-placeholder: #6b7280;
    --at-input-hover-border: #3a4250;
    --at-surface: #161a22;
    --at-surface-muted: #12151c;
    --at-border-soft: #2a2e38;
    --at-text-muted: #9ca3af;
  }
}

.banner {
  padding: 6px 10px 0;
}

/* ===== Unified title bar (menu + window controls) ===== */
.titlebar {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: stretch;
  background: #f8f9fb;
  border-bottom: 1px solid #e5e7eb;
  user-select: none;
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 10px;
  flex-shrink: 0;
}

.titlebar-left .brand-name {
  font-size: 13px;
  font-weight: 700;
  color: #111827;
  margin-right: 4px;
}

.brand-mark {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  object-fit: cover;
  display: block;
  flex-shrink: 0;
  background: transparent;
}

.brand-mark.sm {
  width: 18px;
  height: 18px;
  border-radius: 4px;
}

.menubar-menus {
  display: flex;
  align-items: center;
  gap: 1px;
}

.titlebar-drag {
  flex: 1;
  min-width: 24px;
  cursor: default;
}

.titlebar-right {
  display: flex;
  align-items: stretch;
  flex-shrink: 0;
}

.win-btn {
  width: 46px;
  border: none;
  background: transparent;
  color: #4b5563;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;

  &:hover { background: rgba(0, 0, 0, 0.06); color: #111827; }
  &.win-close:hover {
    background: #e81123;
    color: #fff;
  }
}

.menu-item {
  position: relative;
}

.menu-label {
  display: inline-flex;
  align-items: center;
  height: 24px;
  padding: 0 10px;
  border-radius: 4px;
  font-size: 12px;
  color: #374151;
  cursor: default;
}

.menu-item:hover .menu-label,
.menu-item.open .menu-label {
  background: rgba(0, 0, 0, 0.06);
  color: #111827;
}

.menu-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  min-width: 220px;
  margin-top: 4px;
  padding: 4px 0;
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  z-index: 1000;
}

.menu-option {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  border: none;
  background: transparent;
  padding: 7px 14px;
  font-size: 12px;
  color: #1f2937;
  cursor: pointer;
  text-align: left;

  &:hover:not(:disabled) {
    background: rgba(255, 108, 55, 0.1);
    color: #ff6c37;
  }
  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
}

.menu-shortcut {
  font-size: 11px;
  color: #9ca3af;
  font-family: ui-monospace, Menlo, Consolas, monospace;
}

.menu-divider {
  height: 1px;
  margin: 4px 8px;
  background: #eef0f3;
}

.shortcut-list {
  list-style: none;
  margin: 0;
  padding: 0;

  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
    border-bottom: 1px solid #eef0f3;
    font-size: 13px;
    color: #374151;

    &:last-child { border-bottom: none; }
  }

  kbd {
    display: inline-block;
    min-width: 22px;
    padding: 2px 6px;
    margin: 0 2px;
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    background: #f3f4f6;
    font-size: 11px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    text-align: center;
  }
}

.settings-tabs {
  ::v-deep .el-tabs__header { margin-bottom: 12px; }
  ::v-deep .el-tabs__item.is-active { color: $orange; }
  ::v-deep .el-tabs__active-bar { background: $orange; }
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 0;
  border-bottom: 1px solid #eef0f3;

  &:last-child { border-bottom: none; }
  &.stacked {
    flex-direction: column;
    align-items: stretch;
  }
}

.settings-label {
  min-width: 0;
  strong {
    display: block;
    font-size: 13px;
    color: #111827;
    margin-bottom: 2px;
  }
  p {
    margin: 0;
    font-size: 12px;
    color: #9ca3af;
  }
  code {
    color: $orange;
    background: #fff7ed;
    padding: 0 4px;
    border-radius: 3px;
  }
}

.settings-path {
  display: block;
  margin-top: 8px;
  padding: 8px 10px;
  background: #f3f4f6;
  border-radius: 4px;
  font-size: 11px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  color: #374151;
  word-break: break-all;
  line-height: 1.45;
}

.settings-meta {
  font-size: 12px;
  color: #6b7280;
}

.settings-about {
  text-align: center;
  padding: 16px 0 8px;

  .brand-mark {
    margin: 0 auto 10px;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 18px;
  }
  p {
    margin: 0 0 8px;
    color: #6b7280;
    font-size: 13px;
  }
}

.theme-dark {
  .settings-label strong { color: #e5e7eb; }
  .settings-path {
    background: #1c212b;
    color: #d1d5db;
  }
  .settings-row { border-bottom-color: #2a2e38; }
}

/* ===== Tool bar (light by default) ===== */
.topbar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 12px;
  background: #ffffff;
  color: #1f2937;
  border-bottom: 1px solid #e5e7eb;
}

.topbar-center,
.topbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.topbar-center {
  flex: 1;
  min-width: 0;
}

.topbar-right {
  flex-shrink: 0;
}

.tb-select {
  ::v-deep .el-input__inner {
    background: #f3f4f6;
    border-color: #e5e7eb;
    color: #1f2937;
    height: 28px;
    line-height: 28px;
  }
  ::v-deep .el-input__inner:hover,
  ::v-deep .el-input__inner:focus {
    border-color: #d1d5db;
  }
}

.ws-select { width: 140px; }
.env-select { width: 150px; }

.tb-env-icon {
  color: #9ca3af;
  display: inline-flex;
}

.tb-divider {
  width: 1px;
  height: 18px;
  background: #e5e7eb;
  margin: 0 4px;
}

.icon-btn {
  border: none;
  background: transparent;
  color: #6b7280;
  width: 28px;
  height: 28px;
  border-radius: 4px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;

  &:hover { background: rgba(0, 0, 0, 0.06); color: #111827; }
  &.sm { width: 26px; height: 26px; }
  &.xs { width: 22px; height: 22px; font-size: 12px; }
  &.danger:hover { color: #ef4444; background: rgba(239, 68, 68, 0.1); }
  &:disabled { opacity: 0.4; cursor: not-allowed; }
}

.ghost-btn {
  border: 1px solid #e5e7eb;
  background: #fff;
  color: #4b5563;
  height: 28px;
  padding: 0 10px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 5px;

  &:hover:not(:disabled) {
    border-color: $orange;
    color: $orange;
  }
  &:disabled { opacity: 0.4; cursor: not-allowed; }
}

.runtime-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: #f3f4f6;
  color: #6b7280;
  border: 1px solid #e5e7eb;
  &.ok {
    background: rgba(16, 185, 129, 0.1);
    color: #059669;
    border-color: rgba(16, 185, 129, 0.25);
  }
}

.theme-toggle {
  color: #6b7280;
}

/* ===== Main ===== */
.main-panes {
  flex: 1;
  min-height: 0;
  display: flex;
}

/* ===== Left sidebar (light by default) ===== */
.pane-left {
  width: 268px;
  flex-shrink: 0;
  background: #f3f4f6;
  color: #374151;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-right: 1px solid #e5e7eb;
}

.pane-left .icon-btn {
  color: #6b7280;

  &:hover {
    background: rgba(0, 0, 0, 0.06);
    color: #111827;
  }
  &.danger:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }
}

.left-seg {
  display: flex;
  padding: 8px 8px 0;
  gap: 2px;
}

.seg-btn {
  flex: 1;
  height: 30px;
  border: none;
  background: transparent;
  color: #6b7280;
  font-size: 12px;
  font-weight: 600;
  border-radius: 4px 4px 0 0;
  cursor: pointer;
  letter-spacing: 0.2px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;

  &.active {
    background: #fff;
    color: #111827;
    box-shadow: inset 0 -2px 0 $orange;
  }
  &:hover:not(.active) { color: #111827; }
}

.pane-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-bottom: 1px solid #e5e7eb;
  background: #fff;
}

.pane-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: #9ca3af;
}

.pane-actions { display: flex; gap: 2px; }

.tree {
  flex: 1;
  overflow: auto;
  padding: 6px 4px;
  background: #fff;

  ::v-deep .el-tree {
    background: transparent;
    color: #374151;
  }
  ::v-deep .el-tree-node__content {
    height: 32px;
    border-radius: 4px;
    margin: 1px 4px;
    &:hover { background: #f3f4f6; }
  }
  ::v-deep .el-tree-node.is-current > .el-tree-node__content {
    background: rgba(255, 108, 55, 0.12);
    color: #111827;
  }
  ::v-deep .el-tree-node__expand-icon { color: #9ca3af; }
  ::v-deep .el-tree-node__expand-icon.is-leaf { color: transparent; }
}

.tree-node {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-right: 4px;
  font-size: 13px;
  min-width: 0;
}

.tree-label {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
}

.tree-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.folder-icon { color: #f59e0b; }

.tree-actions {
  opacity: 0;
  display: flex;
  flex-shrink: 0;
}
.tree-node:hover .tree-actions { opacity: 1; }

/* Method badges (Postman palette) */
.method-badge {
  font-size: 10px;
  font-weight: 800;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  letter-spacing: 0.3px;
  min-width: 32px;
  text-align: left;
  flex-shrink: 0;

  &.m-get { color: #10b981; }
  &.m-post { color: #f59e0b; }
  &.m-put { color: #3b82f6; }
  &.m-patch { color: #8b5cf6; }
  &.m-delete, &.m-del { color: #ef4444; }
  &.m-head, &.m-options, &.m-opt, &.m-pat { color: #94a3b8; }
}

.history-list {
  flex: 1;
  overflow: auto;
  padding: 6px;
  background: #fff;
}

.history-item {
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 2px;

  &:hover { background: #f3f4f6; }

  .history-top {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }
  .duration { font-size: 11px; color: #9ca3af; }
  .history-url {
    font-size: 12px;
    color: #374151;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .history-time { margin-top: 2px; font-size: 11px; color: #9ca3af; }
}

.history-empty {
  color: #9ca3af;
  text-align: center;
  padding: 32px 8px;
  font-size: 13px;
}

.status-chip {
  font-size: 11px;
  font-weight: 700;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  &.s-ok { color: #10b981; }
  &.s-err { color: #ef4444; }
  &.s-warn { color: #f59e0b; }
  &.s-muted { color: #6b7280; }
}

/* ===== Right pane ===== */
.pane-right {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: $panel;
}

.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #9ca3af;
  gap: 8px;
}

.empty-illu {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: linear-gradient(135deg, rgba(255, 108, 55, 0.15), rgba(255, 108, 55, 0.05));
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: $orange;
  margin-bottom: 8px;
}

.empty-title { font-size: 16px; font-weight: 600; color: #374151; margin: 0; }
.empty-sub { font-size: 13px; margin: 0 0 12px; }
.empty-cta { padding: 0 20px; height: 36px; }

.req-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 14px 0;
}

.req-name {
  max-width: 280px;
  ::v-deep .el-input__inner {
    border: none;
    border-bottom: 1px solid transparent;
    border-radius: 0;
    font-weight: 600;
    font-size: 14px;
    padding-left: 0;
    &:hover, &:focus { border-bottom-color: $border; }
  }
}

.shortcut-hint {
  margin-left: auto;
  font-size: 11px;
  color: #9ca3af;
}

/* URL bar — signature Postman look */
.url-bar-wrap {
  margin: 8px 14px 12px;
}

.url-bar {
  display: flex;
  align-items: stretch;
  border: 1px solid $border;
  border-radius: 6px;
  overflow: hidden;
  background: #fff;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);

  .method-select {
    width: 110px;
    flex-shrink: 0;

    ::v-deep .el-input__inner {
      border: none;
      border-right: 1px solid $border;
      border-radius: 0;
      font-weight: 800;
      font-size: 13px;
      font-family: ui-monospace, Menlo, Consolas, monospace;
      height: 40px;
      line-height: 40px;
      padding-left: 12px;
      background: #fafafa;
    }
  }

  &.method-get .method-select ::v-deep .el-input__inner,
  .method-select.method-get ::v-deep .el-input__inner { color: #10b981; }
  .method-select.method-post ::v-deep .el-input__inner { color: #f59e0b; }
  .method-select.method-put ::v-deep .el-input__inner { color: #3b82f6; }
  .method-select.method-patch ::v-deep .el-input__inner { color: #8b5cf6; }
  .method-select.method-delete ::v-deep .el-input__inner { color: #ef4444; }

  .url-input {
    flex: 1;
    ::v-deep .el-input__inner {
      border: none;
      border-radius: 0;
      height: 40px;
      line-height: 40px;
      font-size: 13px;
      font-family: ui-monospace, Menlo, Consolas, monospace;
    }
  }
}

.send-btn {
  border: none;
  background: $orange;
  color: #fff;
  font-weight: 700;
  font-size: 13px;
  padding: 0 18px;
  cursor: pointer;
  letter-spacing: 0.3px;
  transition: background 0.15s;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;

  &:hover:not(:disabled) { background: #ff5722; }
  &:disabled { opacity: 0.7; cursor: wait; }
}

.cancel-btn {
  border: none;
  background: #c62828;
  color: #fff;
  font-weight: 700;
  font-size: 13px;
  padding: 0 18px;
  cursor: pointer;
  letter-spacing: 0.3px;
  transition: background 0.15s;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 96px;

  &:hover { background: #b71c1c; }
}

.save-btn {
  border: none;
  border-left: 1px solid rgba(255, 255, 255, 0.25);
  background: #e85a28;
  color: #fff;
  font-weight: 600;
  font-size: 13px;
  padding: 0 16px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;

  &:hover:not(:disabled) { background: #d14e20; }
  &:disabled { opacity: 0.7; cursor: wait; }
}

.loadtest-btn {
  border: 1px solid #e5e7eb;
  background: #fff;
  color: #374151;
  font-weight: 600;
  font-size: 13px;
  padding: 0 14px;
  margin-left: 8px;
  border-radius: 4px;
  cursor: pointer;
  height: 32px;

  &:hover:not(:disabled) {
    border-color: $orange;
    color: $orange;
  }
  &:disabled { opacity: 0.55; cursor: not-allowed; }
}

.loadtest-form {
  margin-bottom: 8px;
  ::v-deep .el-form-item { margin-right: 16px; }
}

.loadtest-result {
  margin-top: 8px;
  padding-top: 12px;
  border-top: 1px solid #eef0f3;
}

.loadtest-stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin-bottom: 12px;
}

.stat {
  background: #f9fafb;
  border-radius: 6px;
  padding: 10px 12px;
  strong {
    display: block;
    font-size: 16px;
    color: #111827;
    margin-top: 2px;
  }
  &.ok strong { color: #059669; }
  &.err strong { color: #dc2626; }
}

.stat-label {
  font-size: 12px;
  color: #9ca3af;
}

.status-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}

.status-chip {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 999px;
  background: #f3f4f6;
  color: #374151;
  &.s-ok { background: #ecfdf5; color: #059669; }
  &.s-err { background: #fef2f2; color: #dc2626; }
  &.s-warn { background: #fffbeb; color: #d97706; }
}

.error-samples {
  margin-top: 10px;
  ul {
    margin: 6px 0 0;
    padding-left: 18px;
    color: #6b7280;
    font-size: 12px;
  }
}

.editor-split {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  margin-top: 4px;
  &.is-splitting {
    cursor: row-resize;
    user-select: none;
  }
}

.req-section {
  min-height: 120px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.split-resizer {
  flex-shrink: 0;
  height: 6px;
  margin: 0;
  cursor: row-resize;
  position: relative;
  background: transparent;
  z-index: 2;

  &::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    top: 2px;
    height: 1px;
    background: $border;
  }

  &:hover,
  &:active {
    &::before {
      background: $orange;
      height: 2px;
      top: 2px;
    }
    .split-grip {
      opacity: 1;
      background: $orange;
    }
  }
}

.split-grip {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 36px;
  height: 3px;
  border-radius: 2px;
  background: #c4c9d2;
  opacity: 0.7;
  pointer-events: none;
  transition: opacity 0.15s, background 0.15s;
}

.req-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 0 10px;

  ::v-deep .el-tabs__header {
    margin: 0;
    .el-tabs__item {
      height: 36px;
      line-height: 36px;
      font-size: 13px;
      font-weight: 500;
      color: #6b7280;
      &.is-active { color: $orange; font-weight: 600; }
    }
    .el-tabs__active-bar { background: $orange; height: 2px; }
    .el-tabs__nav-wrap::after { height: 1px; background: $border; }
  }
  ::v-deep .el-tabs__content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 4px 12px;
  }
}

.body-toolbar {
  margin-bottom: 10px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.body-type-group {
  ::v-deep .el-radio {
    margin-right: 14px;
    .el-radio__label {
      font-size: 12px;
      padding-left: 6px;
      color: #4b5563;
    }
    &.is-checked .el-radio__label {
      color: $orange;
      font-weight: 600;
    }
    .el-radio__inner {
      width: 14px;
      height: 14px;
    }
    &.is-checked .el-radio__inner {
      border-color: $orange;
      background: $orange;
    }
  }
}

.raw-lang-select {
  width: 120px;
}

.beautify-btn {
  border: none;
  background: transparent;
  color: $orange;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  padding: 4px 6px;
  margin-left: auto;
  &:hover { text-decoration: underline; }
}

.binary-body {
  padding: 8px 0;
}

.binary-file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px dashed $border;
  border-radius: 6px;
  background: #f8fafc;
}

.binary-name {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: #6b7280;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pick-btn {
  border: 1px solid $border;
  background: #fff;
  color: #374151;
  font-size: 12px;
  padding: 4px 10px;
  border-radius: 4px;
  cursor: pointer;
  &:hover { border-color: $orange; color: $orange; }
}

.clear-btn {
  border: none;
  background: transparent;
  color: #9ca3af;
  cursor: pointer;
  padding: 2px;
  display: inline-flex;
  &:hover { color: #ef4444; }
}

.hidden-file {
  display: none;
}

.graphql-body {
  .label {
    font-size: 12px;
    font-weight: 600;
    color: #6b7280;
    margin-bottom: 6px;
  }
}

.body-none {
  color: #9ca3af;
  padding: 28px 0;
  text-align: center;
  font-size: 13px;
}

.body-textarea {
  ::v-deep .el-textarea__inner {
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.55;
    background: var(--at-surface-muted, #f8fafc);
    border-color: var(--at-input-border, #dcdfe6);
    color: var(--at-input-color, #1f2937);
  }
}

.script-hint {
  margin-bottom: 8px;
  color: #9ca3af;
  font-size: 12px;
}

.mock-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
  flex-wrap: wrap;
  .label { font-size: 12px; color: #6b7280; }
}

/* Response */
.resp-section {
  flex: 1;
  min-height: 120px;
  display: flex;
  flex-direction: column;
  background: #fafafa;
  overflow: hidden;
}

.response-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-bottom: 1px solid $border;
  flex-shrink: 0;
  min-height: 40px;
  flex-wrap: wrap;
}

.resp-label {
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #6b7280;
}

.status-big {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-weight: 800;
  font-size: 14px;
  em {
    font-style: normal;
    font-weight: 500;
    font-size: 12px;
    margin-left: 4px;
    opacity: 0.85;
  }
  &.s-ok { color: #10b981; }
  &.s-err { color: #ef4444; }
  &.s-warn { color: #f59e0b; }
  &.s-muted { color: #6b7280; }
}

.meta-pill {
  font-size: 12px;
  color: #6b7280;
  background: #fff;
  border: 1px solid $border;
  border-radius: 4px;
  padding: 2px 8px;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: 4px;

  &.type-pill {
    max-width: 180px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 11px;
  }

  &.sending-pill {
    color: $orange;
    border-color: rgba(255, 108, 55, 0.35);
    background: rgba(255, 108, 55, 0.08);
  }
}

.meta-spacer { flex: 1; min-width: 8px; }

.meta-icon-btn {
  border: 1px solid $border;
  background: #fff;
  color: #6b7280;
  width: 26px;
  height: 26px;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;

  &:hover:not(:disabled) {
    color: $orange;
    border-color: rgba(255, 108, 55, 0.45);
  }
  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
}

.meta-muted { font-size: 12px; color: #9ca3af; }

.response-url {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 14px 8px;
  border-bottom: 1px solid $border;
  flex-shrink: 0;
  min-width: 0;

  .url-label {
    font-size: 10px;
    font-weight: 700;
    color: #9ca3af;
    letter-spacing: 0.4px;
    flex-shrink: 0;
  }
  .url-text {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    color: #6b7280;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.mock-chip {
  font-size: 10px;
  font-weight: 800;
  background: #fef3c7;
  color: #d97706;
  padding: 2px 6px;
  border-radius: 3px;
}

.resp-error { margin: 8px 14px 0; flex-shrink: 0; }

.resp-tabs {
  flex: 1;
  min-height: 0;
  padding: 0 10px 10px;
  display: flex;
  flex-direction: column;
  position: relative;

  ::v-deep .el-tabs__header {
    margin: 0;
    flex-shrink: 0;
    .el-tabs__item {
      height: 34px;
      line-height: 34px;
      font-size: 13px;
      &.is-active { color: $orange; }
    }
    .el-tabs__active-bar { background: $orange; }
  }
  ::v-deep .el-tabs__content {
    position: absolute;
    top: 42px;
    left: 10px;
    right: 10px;
    bottom: 10px;
    overflow: hidden;
    padding-top: 0;
  }
  ::v-deep .el-tab-pane {
    height: 100%;
    min-height: 0;
    overflow: auto;
  }
}

.headers-table {
  height: 100%;
  overflow: auto;
  border: 1px solid $border;
  border-radius: 6px;
  background: #fff;
}

.headers-head,
.headers-row {
  display: grid;
  grid-template-columns: minmax(120px, 28%) 1fr 28px;
  gap: 8px;
  align-items: start;
  padding: 8px 10px;
  font-size: 12px;
}

.headers-head {
  position: sticky;
  top: 0;
  background: #f8fafc;
  border-bottom: 1px solid $border;
  font-weight: 700;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  font-size: 11px;
  z-index: 1;
}

.headers-row {
  border-bottom: 1px solid #eef0f3;
  font-family: ui-monospace, Menlo, Consolas, monospace;

  &:last-child { border-bottom: none; }
  &:hover { background: #fafafa; }

  .h-key {
    color: #334155;
    font-weight: 600;
    word-break: break-all;
  }
  .h-val {
    color: #64748b;
    word-break: break-all;
    white-space: pre-wrap;
  }
}

.response-empty {
  color: #9ca3af;
  padding: 40px;
  text-align: center;
  font-size: 13px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  flex: 1;

  &.compact {
    padding: 28px 12px;
    flex: none;
  }

  .at-icon { color: #d1d5db; opacity: 0.85; }
  p { margin: 0; }

  .loading-hint {
    font-size: 12px;
    color: #c4c9d2;
  }
}

.response-loading .at-icon {
  color: $orange;
}

.env-hint {
  margin-bottom: 12px;
  color: #6b7280;
  font-size: 13px;
  code {
    background: #f3f4f6;
    padding: 1px 6px;
    border-radius: 3px;
    color: $orange;
  }
}

.test-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 4px;
  margin-bottom: 6px;
  font-size: 13px;

  &.pass { background: #ecfdf5; color: #059669; }
  &.fail { background: #fef2f2; color: #dc2626; }
  .test-err { color: #9ca3af; font-weight: 400; }
}

/* ===== Dark theme ===== */
.theme-dark {
  background: #0f1115;
  color: #e5e7eb;

  .topbar {
    background: #212121;
    color: #e5e7eb;
    border-bottom-color: #111;
  }

  .titlebar {
    background: #1a1a1a;
    border-bottom-color: #111;
  }

  .titlebar-left .brand-name { color: #e5e7eb; }

  .win-btn {
    color: #c8cdd5;
    &:hover { background: rgba(255, 255, 255, 0.08); color: #fff; }
    &.win-close:hover { background: #e81123; color: #fff; }
  }

  .menu-label { color: #d1d5db; }
  .menu-item:hover .menu-label,
  .menu-item.open .menu-label {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .menu-dropdown {
    background: #262626;
    border-color: #3a3a3a;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
  }

  .menu-option {
    color: #e5e7eb;
    &:hover:not(:disabled) {
      background: rgba(255, 108, 55, 0.15);
      color: #ff8f66;
    }
  }

  .menu-shortcut { color: #6b7280; }
  .menu-divider { background: #3a3a3a; }

  .tb-select {
    ::v-deep .el-input__inner {
      background: #2a2a2a;
      border-color: #3a3a3a;
      color: #e5e7eb;
    }
  }

  .tb-divider { background: #3a3a3a; }

  .icon-btn {
    color: #c8cdd5;
    &:hover { background: rgba(255, 255, 255, 0.08); color: #fff; }
    &.danger:hover {
      color: #f87171;
      background: rgba(248, 113, 113, 0.12);
    }
  }

  .ghost-btn {
    border-color: #3a3a3a;
    background: transparent;
    color: #d1d5db;
  }

  .runtime-chip {
    background: #374151;
    color: #9ca3af;
    border-color: transparent;
    &.ok {
      background: rgba(16, 185, 129, 0.15);
      color: #34d399;
      border-color: transparent;
    }
  }

  .theme-toggle { color: #e5e7eb; }

  .pane-left {
    background: $sidebar;
    color: $sidebar-text;
    border-right-color: #111;
  }

  .pane-left .icon-btn {
    color: #c8cdd5;
    &:hover {
      background: rgba(255, 255, 255, 0.08);
      color: #fff;
    }
    &.danger:hover {
      color: #f87171;
      background: rgba(248, 113, 113, 0.12);
    }
  }

  .seg-btn {
    color: #9ca3af;
    &.active {
      background: #262a33;
      color: #fff;
    }
    &:hover:not(.active) { color: #e5e7eb; }
  }

  .pane-header {
    background: transparent;
    border-bottom-color: #2a2e38;
  }

  .tree {
    background: transparent;
    ::v-deep .el-tree { color: $sidebar-text; }
    ::v-deep .el-tree-node__content:hover {
      background: rgba(255, 255, 255, 0.06);
    }
    ::v-deep .el-tree-node.is-current > .el-tree-node__content {
      background: rgba(255, 108, 55, 0.18);
      color: #fff;
    }
    ::v-deep .el-tree-node__expand-icon { color: #6b7280; }
  }

  .history-list { background: transparent; }
  .history-item {
    &:hover { background: rgba(255, 255, 255, 0.06); }
    .history-url { color: #d1d5db; }
  }
  .history-empty { color: #6b7280; }

  .pane-right { background: #161a22; }
  .req-section { border-color: #2a2e38; background: #161a22; }
  .binary-file-row {
    background: #1c212b;
    border-color: #2a2e38;
  }
  .pick-btn {
    background: #1c212b;
    border-color: #2a2e38;
    color: #e5e7eb;
  }
  .body-type-group {
    ::v-deep .el-radio .el-radio__label { color: #9ca3af; }
    ::v-deep .el-radio.is-checked .el-radio__label { color: $orange; }
  }
  .resp-section { background: #12151c; }
  .split-resizer {
    &::before { background: #2a2e38; }
    &:hover::before,
    &:active::before { background: $orange; }
    .split-grip { background: #4b5563; }
    &:hover .split-grip,
    &:active .split-grip { background: $orange; }
  }
  .meta-pill {
    background: #1c212b;
    border-color: #2a2e38;
    color: #9ca3af;
  }
  .meta-icon-btn {
    background: #1c212b;
    border-color: #2a2e38;
    color: #9ca3af;
    &:hover:not(:disabled) {
      color: #ff8f66;
      border-color: rgba(255, 108, 55, 0.45);
    }
  }
  .response-meta,
  .response-url {
    border-color: #2a2e38;
  }
  .response-url .url-text { color: #9ca3af; }
  .headers-table {
    background: #1c212b;
    border-color: #2a2e38;
  }
  .headers-head {
    background: #161a22;
    border-color: #2a2e38;
    color: #9ca3af;
  }
  .headers-row {
    border-color: #2a2e38;
    &:hover { background: rgba(255, 255, 255, 0.03); }
    .h-key { color: #e5e7eb; }
    .h-val { color: #9ca3af; }
  }
  .url-bar {
    background: #1c212b;
    border-color: #2a2e38;
    .method-select ::v-deep .el-input__inner {
      background: #1c212b;
      border-color: #2a2e38;
      color: inherit;
    }
    .url-input ::v-deep .el-input__inner {
      background: #1c212b;
      color: #e5e7eb;
    }
  }
  .loadtest-btn {
    background: #1c212b;
    border-color: #2a2e38;
    color: #e5e7eb;
  }
  .loadtest-result { border-top-color: #2a2e38; }
  .stat {
    background: #1c212b;
    strong { color: #e5e7eb; }
  }
  .status-chip { background: #1c212b; color: #d1d5db; }
  .req-name ::v-deep .el-input__inner {
    background: transparent;
    color: #e5e7eb;
  }
  .empty-title { color: #e5e7eb; }
  .body-textarea ::v-deep .el-textarea__inner {
    background: var(--at-input-bg);
    border-color: var(--at-input-border);
    color: var(--at-input-color);
  }
  .env-hint code { background: #1c212b; }
  .test-row.pass { background: rgba(16, 185, 129, 0.12); }
  .test-row.fail { background: rgba(239, 68, 68, 0.12); }

  ::v-deep .el-tabs__item { color: #9ca3af; }
  ::v-deep .el-tabs__item.is-active { color: $orange; }
  ::v-deep .el-tabs__nav-wrap::after { background: #2a2e38; }

  /* Config editors: Params / Headers / Body / Scripts / Mock */
  ::v-deep .el-input__inner,
  ::v-deep .el-textarea__inner {
    background-color: var(--at-input-bg) !important;
    border-color: var(--at-input-border) !important;
    color: var(--at-input-color) !important;

    &::placeholder {
      color: var(--at-input-placeholder) !important;
    }

    &:hover,
    &:focus {
      border-color: var(--at-input-hover-border) !important;
    }
  }

  ::v-deep .el-input.is-disabled .el-input__inner,
  ::v-deep .el-textarea.is-disabled .el-textarea__inner {
    background-color: #161a22 !important;
    color: #6b7280 !important;
  }

  ::v-deep .el-input-number {
    .el-input__inner {
      background-color: var(--at-input-bg) !important;
      border-color: var(--at-input-border) !important;
      color: var(--at-input-color) !important;
    }
    .el-input-number__decrease,
    .el-input-number__increase {
      background: #161a22;
      border-color: var(--at-input-border);
      color: #9ca3af;
      &:hover { color: $orange; }
    }
  }

  ::v-deep .el-select .el-input__inner {
    background-color: var(--at-input-bg) !important;
  }

  ::v-deep .el-autocomplete {
    width: 100%;
  }

  ::v-deep .kv-editor,
  ::v-deep .form-data-editor {
    .kv-head,
    .fd-head {
      border-bottom-color: var(--at-border-soft);
      color: #9ca3af;
    }
  }

  ::v-deep .form-data-editor {
    .file-name {
      background: var(--at-input-bg);
      border-color: var(--at-input-border);
      color: var(--at-input-color);
    }
    .pick-btn {
      background: var(--at-input-bg);
      border-color: var(--at-input-border);
      color: var(--at-input-color);
    }
    .add-btn {
      border-color: #3a4250;
      color: #9ca3af;
    }
  }

  ::v-deep .el-radio__label {
    color: #9ca3af;
  }
  ::v-deep .el-radio.is-checked .el-radio__label {
    color: $orange;
  }

  ::v-deep .el-switch__label {
    color: #9ca3af;
  }
}
</style>

<style lang="scss">
/* Dialogs append to body — need document-level dark class */
.apitest-theme-dark {
  .apitest-dlg {
    background: #1c212b !important;
    border: 1px solid #2a2e38;

    .el-dialog__header {
      border-bottom: 1px solid #2a2e38;
    }
    .el-dialog__title {
      color: #e5e7eb;
    }
    .el-dialog__body {
      color: #d1d5db;
    }
    .el-dialog__footer {
      border-top: 1px solid #2a2e38;
    }
    .el-dialog__headerbtn .el-dialog__close {
      color: #9ca3af;
    }

    .el-input__inner,
    .el-textarea__inner {
      background-color: #161a22 !important;
      border-color: #2a2e38 !important;
      color: #e5e7eb !important;

      &::placeholder {
        color: #6b7280 !important;
      }
    }

    .el-input-number {
      .el-input__inner {
        background-color: #161a22 !important;
        border-color: #2a2e38 !important;
        color: #e5e7eb !important;
      }
      .el-input-number__decrease,
      .el-input-number__increase {
        background: #12151c;
        border-color: #2a2e38;
        color: #9ca3af;
      }
    }

    .el-button--default {
      background: #161a22;
      border-color: #2a2e38;
      color: #e5e7eb;
      &:hover {
        border-color: #ff6c37;
        color: #ff6c37;
      }
    }

    .kv-editor .el-input__inner,
    .form-data-editor .el-input__inner {
      background-color: #161a22 !important;
      border-color: #2a2e38 !important;
      color: #e5e7eb !important;
    }

    .env-hint { color: #9ca3af; }
    .env-hint code {
      background: #12151c;
      color: #d1d5db;
    }
  }

  .el-select-dropdown,
  .el-autocomplete-suggestion {
    background: #1c212b;
    border-color: #2a2e38;

    .el-select-dropdown__item,
    li {
      color: #e5e7eb;
      &.hover,
      &:hover {
        background: rgba(255, 108, 55, 0.12);
      }
      &.selected {
        color: #ff6c37;
      }
    }
  }

  .el-popper[x-placement] .popper__arrow::after {
    border-bottom-color: #1c212b;
  }
}
</style>
