---
name: vibecoding-engineering
description: 用证据驱动、可追踪、可审查的流程指导 Agent 完成功能、Bug、架构和长期软件工程任务。
metadata:
  short-description: Evidence-driven software engineering for AI agents
---

# Vibecoding Engineering Skill

## 1. 目标

把一次 AI 编程任务从“给出目标，等待 Agent 声称完成”变成可理解、可追踪、可验证、可审查、可回滚、可交接的工程过程：

```text
Intent
  ↓
Research
  ↓
Clarify
  ↓
Understand the code
  ↓
Decision
  ↓
Plan + Acceptance Criteria
  ↓
Implement
  ↓
Observe
  ↓
Verify
  ↓
Evidence
  ↓
Review
  ↓
Done
```

Agent 的职责不是只生成代码，而是帮助建立能够及时发现、定位、纠正错误的工作系统。任何“完成”声明都必须由可独立检查的证据支持。

## 2. 适用范围

对以下任务使用本 Skill，并按任务规模缩放深度：

- 新功能、产品流程和 API；
- Bug 诊断与修复；
- 架构设计、模块拆分和重构；
- UI 调整、交互实现和视觉还原；
- 测试补充、代码 Review 和长期任务接管；
- 需要跨文件、跨组件或跨外部系统追踪的工作。

极小且无风险的单行修改可以缩短流程，但不得跳过必要的理解、范围控制和验证。

## 3. 硬规则

### 3.1 禁止许愿式编程

不要把以下模糊指令直接当作完整规格：

- “把这个功能做好。”
- “把这个 Bug 修掉。”
- “优化一下架构。”
- “按参考项目实现。”
- “你自己检查，没问题就提交。”

不要替请求方自行补全关键需求、架构、边界、验收或风险。必须把任务转化为可观察的中间产物：调研结论、事实、决策、计划、验收标准、修改范围和验证证据。

### 3.2 先读代码，再改代码

遵循 `Read Before Write` 与 `Understand Before Modify`。在理解入口、调用链、数据流、现有测试和影响范围之前，不得开始实质性实现。

### 3.3 模糊就澄清

只要关键行为、边界、数据、约束或完成标准存在多种合理解释，就暂停实现并提问。不得用“通常应该是……”代替用户决策。

### 3.4 不懂不接受

如果方案、架构或修改理由无法用自己的话解释清楚，就不能批准实施。Agent 必须解释职责、数据流、依赖、收益、复杂度、风险和替代方案，并指出仍不确定的地方。

### 3.5 事实优先于叙述

代码、测试、Git Diff、日志、实际运行行为、浏览器状态和截图是验证材料；Agent 的“已经完成”“应该没问题”只是 Claim，不是 Evidence。

### 3.6 严格控制范围

禁止借机重构无关模块、顺手改风格、替换依赖或扩大 API。发现相邻问题时记录为独立 Issue/任务，除非获得明确授权。

### 3.7 复杂度必须匹配问题

优先采用当前项目中最简单、可解释、可验证的方案。不要为了体现模式而增加层、抽象或依赖；也不要为了省事把不相关职责塞进巨型文件。

### 3.8 Handoff 不是权威结论

交接文档、旧对话和 Agent 推理都是历史信息。新 Agent 必须用真实代码、Git 状态、测试和日志核验，不得盲信。

## 4. 何时停止并澄清

遇到以下任一情况，停止实现，先报告已知事实、未知点和需要的决策：

- 目标用户、成功行为或明确的“不做什么”不清楚；
- 成功、失败、空数据、加载、权限、重复提交或超时行为未定义；
- Spec、架构文档、代码和测试互相矛盾；
- 无法确定入口、真实调用链、数据转换或副作用；
- 方案需要新增依赖、改变公共契约、修改数据模型、权限或高风险配置，但没有批准；
- Debug 只有现象，没有可重复步骤或证据；
- 根因仍只是猜测；
- 计划没有可操作的验收标准或测试方法；
- 现有未提交 Diff 使修改归属或基线不清楚；
- 预计修改会超出允许文件或模块；
- Review 发现关键风险尚未解决。

可以继续做只读探索、收集证据和提出选项；不能把探索性猜测伪装成实现。

## 5. 任务开始前检查

开始任何非平凡任务时，依次完成：

1. **分类任务**：Feature、Bug、Refactor、Review、Visual、Long-task 或 Sub-agent。
2. **确认目标与边界**：记录要解决的问题、明确不做的内容、目标用户和成功结果。
3. **检查仓库状态**：确认当前分支、未提交 Diff、最近相关提交、测试基线和是否存在并行修改。
4. **读取导航文档**：至少查看 README、项目规则、相关 Architecture、Spec、API 和近期 ADR；只读取与任务相关的部分。
5. **定位真实代码**：搜索入口、核心函数、数据模型、配置、测试、调用方和被调用方。
6. **识别风险**：标记公共接口、持久化数据、认证授权、并发、外部调用、文件格式和不可逆操作。
7. **建立代码地图**：至少对关键链路形成 File Map、Call Map、Data Flow Map 和 Impact Map。
8. **写出计划与验收标准**：在修改前说明步骤、允许范围、验证方式和停止条件。
9. **确认基线**：尽可能先运行相关测试或记录当前已知失败，避免把旧问题误判为本次回归。

## 6. 调研流程：先调研，后设计，再实现

重要功能、架构或技术选型必须先调研。调研的目的不是收集链接，而是减少重复造轮子、错误设计和过度实现。

### 6.1 竞品与同类产品调研

检查：

- 同类产品是否已经解决相同问题；
- 用户完成目标的核心流程；
- 行业内已经形成的交互惯例；
- 值得借鉴的设计与明显缺陷；
- 当前项目真正需要的深度，是否值得复杂实现。

把“别人怎么做”与“本项目决定怎么做”分开记录。竞品行为是参考，不是自动接受的需求。

### 6.2 开源与技术方案调研

检查 GitHub、官方文档和成熟库中是否存在可复用的：

- 完整项目或库；
- 单个模块、算法、解析器、导出器或 UI 组件；
- 协议实现、数据结构和架构思路。

不要因为只需要一个模块就复制整个项目。对候选方案记录：

- License 是否允许当前使用方式；
- 维护活跃度、最近更新和 Issue 状况；
- 代码质量、安全性和依赖复杂度；
- 与当前技术栈、架构和数据模型的兼容性；
- 二次开发、升级和长期维护成本；
- 接入成本是否高于自己实现。

结论只能是“采用、部分借鉴、暂不采用或自行实现”，并说明理由。

## 7. 需求澄清与认知对齐

### 7.1 必须澄清的问题

至少逐项确认：

**目标**

- 到底要做什么？
- 明确不做什么？
- 谁使用，为什么需要？

**行为**

- 用户每一步操作后发生什么？
- 成功、失败、取消、重试和重复操作如何表现？
- Loading、Empty、Error、Disabled、Permission Denied 如何表现？

**数据**

- 输入、输出和字段是什么？
- 数据从哪里来，谁创建、谁修改、谁消费？
- 生命周期、校验、默认值和兼容性是什么？

**技术**

- 复用现有模块还是新建模块？
- 同步还是异步？
- 是否需要缓存、队列、新依赖或公共接口变化？

**边界**

- 网络失败、超时、空数据、权限不足、并发和重复提交怎么办？
- 第三方或外部系统返回异常怎么办？

**验收**

- 哪些可观察结果出现时才算完成？
- 哪些测试、操作或截图能够证明完成？

### 7.2 澄清循环

使用以下循环，直到关键歧义消失：

```text
发现模糊
  ↓
提出具体问题与可选方案
  ↓
得到决策
  ↓
写入 Spec / ADR / Task
  ↓
继续检查新的歧义
```

不要只问“这样可以吗”。应说明选项、影响、风险和自己的不确定点，让决策可以被审查。

### 7.3 不懂不接受的检查

对于任何设计，要求 Agent 能回答：

- 每一层和每个模块分别负责什么？
- 数据从入口到输出怎样流动？
- 为什么需要该层，为什么不能用更简单的方案？
- 删除某一层会发生什么？
- 哪些地方最容易出错？
- 以后扩展一个相邻功能需要改哪里？
- 该设计引入了哪些额外复杂度和维护成本？

只有在请求方或负责决策的人能够不看原文复述方案、判断取舍并明确批准后，才能实施。

## 8. 上下文来源与 Source of Truth

按任务需要读取相关上下文，但不要用旧对话取代真实项目状态。各类信息的职责如下：

- **批准的 Spec 与 Acceptance Criteria**：定义预期行为和范围；
- **Architecture 与 ADR**：记录已确认的结构和决策理由；
- **真实代码、配置与测试**：描述当前实际实现；
- **Git 分支、Diff、Commit 与 PR**：描述变更边界和历史；
- **运行结果、日志、浏览器状态和截图**：描述可观察行为；
- **Handoff 与对话**：提供历史背景、尚未解决的问题和待核验建议。

如果文档与代码冲突，报告冲突并回到源头核验；不要默默修改文档或代码来掩盖冲突。每个任务都要明确当前采用的 Source of Truth，且不能把 Agent 的自然语言声明当作权威。

## 9. 代码地图与真实执行链

不要求一开始逐行看懂所有代码，但必须让项目不再是黑盒。对关键功能持续维护四张地图。

### 9.1 File Map

回答“每个关键文件负责什么、属于哪一层、对外暴露什么”。

```text
api.py              → HTTP 入口与请求/响应适配
auth_service.py     → 认证业务规则
user_repository.py  → 用户数据访问
token.py            → Token 生成与验证
```

### 9.2 Call Map

回答“谁调用谁、在哪个条件下调用、错误如何返回”。

```text
login()
  ↓
authenticate_user()
  ↓
get_user()
  ↓
verify_password()
  ↓
issue_token()
```

### 9.3 Data Flow Map

回答“数据经过哪些格式、校验、转换和状态变化”。

```text
HTTP JSON
  ↓
Request Model
  ↓
Service
  ↓
Database Model
  ↓
Response DTO
  ↓
JSON
```

### 9.4 Impact Map

回答“修改一个位置，哪些消费者、输出、测试和外部行为可能受影响”。

```text
修改 Markdown Table Parser
          ↓
可能影响
├── DOCX Export
├── PDF Export
├── Preview
└── Table Validation
```

### 9.5 跨文件与跨组件链路

不能只说“A.py 是解析器”，还要追踪输入如何穿过文件、模块、格式和外部处理环节。例如：

```text
Markdown 表格
  ↓
docx_out.py 解析 Markdown
  ↓
生成中间结构与 OOXML 表格
  ↓
生成 .docx
  ↓
Microsoft Word
  ↓
Word Layout Engine 自动排版
```

最终结果异常时，问题可能在输入、Parser、中间结构、OOXML 属性、生成逻辑或外部布局引擎；不要只盯着最后显示结果对应的文件。

### 9.6 架构图必须来自 Code Trace

禁止“看目录树、猜分层、直接画图”。画流程图或架构图前执行：

```text
Explore Codebase
  ↓
Locate Entry Point
  ↓
Trace Functions and Calls
  ↓
Trace Data Transformations
  ↓
Trace Side Effects and External Boundaries
  ↓
Verify Against Code / Tests / Logs
  ↓
Draw Diagram
```

图的粒度随问题变化：

- 项目总览：Frontend → Backend → Database 等高层边界；
- 功能流程：入口 → 函数 → 模块 → 数据转换 → 输出；
- Debug：具体函数、状态、序列化、文件格式和外部布局行为。

如果不能确认 A 是否调用 B，就回到代码搜索和测试验证；不为让图看起来完整而编造箭头。

## 10. 先读代码再提出修改方案

复杂任务先执行 Explore，不修改代码。探索报告至少包括：

- 当前功能流程；
- 入口、核心函数和关键文件；
- 调用链与数据流；
- 数据模型、配置和外部依赖；
- 现有测试与已知限制；
- 与任务有关的代码位置；
- 可能受影响的消费者；
- 仍然不确定的事实；
- 建议的修改范围及理由。

只有探索结果与真实代码相符、关键不确定点已经处理，才进入 Plan 和 Implement。

## 11. 代码分层与架构约束

采用与项目规模匹配的职责边界。常见方向如下：

```text
Presentation / API
        ↓
Application / Service
        ↓
Domain
        ↓
Repository / Data
        ↓
Infrastructure
```

不是所有项目都需要全部层。必须遵守：

- 每个模块有清晰、单一的主要职责；
- 公共能力集中维护，避免复制粘贴；
- 依赖方向明确，避免循环依赖；
- 接口和数据契约清楚；
- 巨型文件、高耦合和职责混杂必须有充分理由；
- 先检查现有分层，再决定新代码放置位置；
- 新抽象必须解决真实问题，并能通过测试验证；
- 不要为了分层而分层，也不要以“项目小”为由让所有逻辑混在一起。

## 12. 文档分层

不要把整个项目塞进一个 README。按职责维护：

- **README**：项目是什么、如何启动、基本导航；
- **ARCHITECTURE**：系统结构、模块边界、数据流和关键组件；
- **SPEC**：功能目标、范围、行为和 Acceptance Criteria；
- **API**：接口输入、输出、错误、契约和兼容性；
- **ADR / DECISIONS**：决策、原因、候选方案和被否决的方案；
- **TASK / PLAN**：当前任务、步骤、状态、风险和验证；
- **AGENT RULES**：允许与禁止的目录、操作、规范和确认点；
- **HANDOFF**：长任务的状态、事实、决策链、尝试记录和下一步。

对话用于消除歧义，文档用于保存长期事实，代码用于保存真实实现。重要决策不能只留在聊天记录里。

## 13. 长会话 Handoff

出现以下信号时不要硬撑：上下文过长、早期决策被遗忘、重复讨论、任务边界漂移、已解决问题反复出现，或 Agent 开始依赖不完整记忆。此时：

1. 记录当前状态并生成 Handoff；
2. 保存已完成的验证结果和未解决问题；
3. 提交或保存一个清晰的当前 Git 状态；
4. 开启新会话；
5. 由新 Agent 重新建立认知后继续。

Handoff 必须回答“为什么走到这里”，不能只写“接下来做 X”。至少包含：

- 任务目标与明确范围；
- Spec、Architecture、ADR、Issue、Test 等 Source of Truth 的位置；
- 已完成、进行中和未完成事项；
- 关键文件、模块、入口和函数；
- 当前 File/Call/Data/Impact Map 的摘要；
- 历史决策链：考虑过什么、为何否决、为何采用当前方案；
- 已尝试方案，尤其是失败方案及失败原因；
- 通过代码、测试、日志或运行结果确认的事实；
- 尚未验证的假设，明确标记为假设；
- 已知 Bug、风险和阻塞点；
- Branch、Commit、PR 和当前 Diff；
- 已运行的测试、通过项、失败项和未测试项；
- 下一步建议，并明确“建议不是不可质疑的结论”。

## 14. 新会话接管

新 Agent 按以下顺序接管，不要直接照抄旧结论：

```text
快速读取项目基础文档与导航
  ↓
根据任务范围定位真实代码
  ↓
阅读入口、调用链、数据流、测试与当前 Diff
  ↓
阅读 Handoff，理解历史与决策链
  ↓
回到代码、Git、测试和日志核验 Handoff
  ↓
输出自己的理解与不确定点
  ↓
请求确认或澄清
  ↓
继续 Plan / Implement / Debug
```

接管报告必须能说明：

- 当前目标是什么；
- 系统现在如何工作；
- 为什么走到当前状态；
- 已完成什么；
- 卡在哪里；
- 哪些是已证实事实；
- 哪些仍是假设；
- 下一步是什么以及如何验证。

## 15. 子代理委派

子代理不是主 Agent 的上下文复制品。先确定角色，再决定传递什么信息。

### 15.1 执行型子代理

当技术决策已经完成、子代理的任务只是按方案实现时，提供足够的决策上下文：

- 任务目标、Spec 和已批准方案；
- Architecture、ADR 和相关真实代码；
- 允许修改、限制修改和禁止修改的范围；
- 接口契约、数据约束和不可改变的行为；
- Acceptance Criteria、测试方法和完成证据格式；
- 已知风险与必须保持的兼容性。

要求返回：

- 修改文件与每个修改的目的；
- 关键实现选择；
- 完整 Diff 摘要；
- 测试、构建、运行和截图结果；
- 未完成项、风险和需要主 Agent 决策的事项。

### 15.2 调查、Debug、Review 型子代理

这类任务的价值来自独立判断。使用“最小充分上下文”：

- 提供客观现象、目标、必要约束、相关代码范围和输出格式；
- 先让子代理读真实代码、测试和日志；
- 先让它独立描述系统如何工作并形成判断；
- 暂缓提供主 Agent 的怀疑、Root Cause、偏好、长篇推理和拟定修复；
- 子代理提交独立结论与证据后，再提供主 Agent 方案；
- 要求双方比较，再用实验、测试或日志验证；
- 未完成对齐前，不授权修改。

刻意避免 Anchoring（锚定效应）与 Confirmation Bias（确认偏误）：不要让子代理只寻找支持主 Agent 预设结论的证据。

委派顺序：

```text
Objective Facts
  ↓
Minimum Sufficient Context
  ↓
Relevant Code Scope
  ↓
Child Reads Code
  ↓
Independent Analysis
  ↓
Evidence and Own Conclusion
  ↓
Compare With Parent Analysis
  ↓
Experiment / Test
  ↓
Decide Whether to Execute
```

共享事实，不急着共享观点；共享约束，不急着共享结论。不要把子代理变成只寻找证据支持主 Agent 预设答案的工具。

## 16. Git 分支、Commit 与 PR

把 Git 当作变更控制与审查系统，而不只是代码备份。

### 16.1 分支

- `main`：稳定、可运行、可作为发布基线的分支；原则上不直接让 Agent 随意修改；
- `feat/<name>`：新功能，如 `feat/user-login`；
- `fix/<name>`：普通 Bug，如 `fix/login-redirect`；
- `hotfix/<name>`：紧急修复，如 `hotfix/payment-timeout`；
- `refactor/<name>`：不改变预期行为的重构，如 `refactor/auth-service`；
- `docs/<name>`：文档修改；
- `chore/<name>`：依赖或工程维护；
- `test/<name>`：主要增加或修改测试。

一个分支对应一个明确任务和一个上下文边界。不同功能、修复和重构分开；发现额外问题时开独立任务和分支。

### 16.2 Commit

Commit 必须小、清晰、单一目的、可回滚，并说明实际变化：

```text
feat(auth): add login endpoint
fix(docx): preserve table column width
test(auth): add expired token case
```

避免 `update stuff` 这类无法审查的描述。大修改前先保存清晰基线，避免把多个不相关目的混进一个 Commit。

用 Git Diff、Log、Revert 和 Cherry-pick 检查、比较或恢复变更；涉及历史重写或可能丢失工作的 Reset 等操作，先确认目标和授权。

### 16.3 PR

不要“AI 写完就直接合并”。标准流程是：

```text
明确任务分支
  ↓
Implement
  ↓
Test and Verify
  ↓
Commit
  ↓
Review Diff
  ↓
PR
  ↓
Independent Review / CI
  ↓
确认验收标准全部满足
  ↓
合并到 main
```

PR 至少说明：

- **Why**：为什么改；
- **What**：改了什么；
- **How**：怎样实现；
- **Risk**：风险与未覆盖边界；
- **Test**：运行了什么、结果如何；
- **Screenshot**：UI 变化时附关键状态截图。

Review 看 Spec、Diff、测试和真实行为，不以 Agent 的自我评价为依据。尽量让独立 Reviewer 不先阅读 Implementer 的长篇辩护。

较大的任务可以按 Planner → Implementer → Reviewer → Tester 分工，但角色分工不能替代主 Agent 的最终验收。

## 17. 规划阶段定义 Done

Plan 必须同时写出验收标准和测试策略：

```text
需求
  ↓
Spec
  ↓
Plan + Acceptance Criteria + Test Strategy
  ↓
Implement
  ↓
Verify
```

验收标准要描述可观察行为，而不是实现愿望。例如登录功能至少应明确：

- 正确账号密码：登录成功并进入目标页面；
- 错误密码：显示明确错误，不发生错误跳转；
- 空用户名：阻止提交并提示原因；
- Token 失效：按约定返回登录流程；
- 刷新页面：登录状态按约定保持或失效；
- 手机尺寸：关键内容、交互和错误状态不出现明显布局问题。

将验证分成四类：

- **Correctness Verification**：逻辑、状态、数据转换和错误处理是否正确；
- **Acceptance Verification**：是否实现了用户真正批准的行为；
- **Regression Verification**：相关已有流程是否仍然正常；
- **Visual Verification**：实际界面、布局、层级、状态和可读性是否正确。

## 18. 截图与浏览器验证

UI 任务不能只凭代码、构建成功或 Agent 描述判断完成。执行：

```text
修改
  ↓
启动真实应用
  ↓
打开目标页面
  ↓
执行真实交互
  ↓
截图关键状态
  ↓
观察并对照需求或参考图
  ↓
修正偏差
  ↓
再次运行与截图
```

适用时覆盖：

- Default；
- Loading；
- Success；
- Error；
- Empty；
- Disabled；
- Mobile；
- Desktop。

有 Figma、设计稿或参考产品时，执行 `Reference Screenshot → Implementation Screenshot → Compare → Fix`。有浏览器能力时还检查 Console、Network、DOM、Runtime Error 和 API Response。

## 19. Debug：证据驱动，禁止 Guess-and-Patch

严格按以下顺序处理：

```text
Reproduce
  ↓
Evidence
  ↓
Narrow
  ↓
Hypothesis
  ↓
Validate
  ↓
Root Cause
  ↓
Minimal Fix
  ↓
Regression
```

### 19.1 每一步的要求

1. **Reproduce**：记录稳定、最小、可重复的步骤、输入、前置状态和实际结果。
2. **Evidence**：收集日志、堆栈、Network、状态快照、截图、数据库/文件结果和相关测试输出。
3. **Narrow**：用调用链、数据流、时间顺序和边界条件缩小到具体模块或转换环节。
4. **Hypothesis**：提出一个或多个可证伪的根因假设，说明每个假设的证据和缺口。
5. **Validate**：用最小实验、日志、测试或对照组验证，不把相关性当因果。
6. **Root Cause**：形成有证据支持的根因判断；如果仍不能支持，继续调查，不进入修复。
7. **Minimal Fix**：只改根因相关位置，避免顺手重构附近模块。
8. **Regression**：重跑 Bug Reproduction Test、既有测试和相关流程测试；确认没有破坏相邻行为。

能做到时，先写或构造一个会失败的 Bug Reproduction Test：

```text
Test Fails
  ↓
Minimal Fix
  ↓
Test Passes
  ↓
Related Regression Tests
```

允许为收集证据添加临时、可识别、可撤销的诊断记录；诊断代码不能被误当成正式修复。

## 20. 修改范围控制

每次任务在 Plan 中写清：

- **Allowed**：允许修改的文件、模块和契约；
- **Restricted**：原则上不动，必须说明理由后再改；
- **Forbidden**：禁止的目录、秘密、不可逆或高风险操作。

尤其对数据库迁移、认证授权、公共 API、持久化格式、第三方集成和破坏性命令设置额外检查。完成后检查：

- Diff 是否只包含任务相关文件；
- 是否出现无关重构、格式化、依赖或命名变化；
- 是否引入未批准的公共行为；
- 是否新增了需要单独审查的风险；
- 是否可以用最小修改解释全部变化。

## 21. Evidence of Done

提交完成声明时，附上适用的证据，而不是只写“完成”：

```text
Acceptance Criteria       ✓ / 部分 / 未完成
Correctness Tests         ✓ / 部分 / 未运行
Acceptance Flow           ✓ / 部分 / 未运行
Regression Tests          ✓ / 部分 / 未运行
Lint / Type Check         ✓ / 部分 / 未运行
Build                     ✓ / 部分 / 未运行
Runtime Verification      ✓ / 部分 / 未运行
Screenshot / Visual Check ✓ / 部分 / 不适用
Console / Network Check   ✓ / 部分 / 不适用
Diff Scope Review         ✓ / 未完成
Independent Review        ✓ / 未完成
```

测试证据按项目适用性选择，可包括 Unit、Integration、API、E2E、Lint、Type Check、Build 和 CI。每项至少注明运行方式、结果、关键输出或证据位置。明确列出未验证项、已知限制、剩余风险和下一步，不要用绿色勾选掩盖空白。

## 22. 完整 Feature SOP

1. 读取任务、项目规则、相关文档与 Git 状态。
2. 做竞品和开源方案调研，记录采用与不采用的理由。
3. 澄清目标、非目标、行为、数据、边界和验收标准。
4. 先读真实代码，定位入口、调用链、数据流、测试和影响范围。
5. 输出 File/Call/Data/Impact Map 与当前理解。
6. 解释候选设计，比较简单方案与复杂方案；对未知点停止并澄清。
7. 更新 Spec/ADR/Task，确定允许修改范围。
8. 创建以任务命名的分支。
9. 写出 Plan、Acceptance Criteria 和 Test Strategy。
10. 按最小范围实现，保持代码分层和现有契约。
11. 运行 Correctness、Acceptance、Regression 验证。
12. 对 UI 启动真实应用，执行关键交互并截图；检查浏览器状态。
13. 检查 Diff，移除无关变更，补充 Evidence of Done。
14. Commit、创建 PR、接受独立 Review 和 CI 检查。
15. 处理 Review 反馈并重新验证；满足验收后才合并。

## 23. 完整 Bug SOP

1. 阅读问题描述、相关代码、测试、最近 Diff 和项目规则。
2. 不修改正式逻辑，先稳定复现。
3. 收集日志、堆栈、Network、状态、截图和测试输出。
4. 根据真实调用链和数据流缩小范围。
5. 提出可证伪假设，明确证据与缺口。
6. 用最小实验或失败测试验证假设。
7. 形成证据支持的 Root Cause；不能支持时停止修复并继续调查。
8. 确认允许修改范围，只做 Minimal Fix。
9. 运行 Bug Reproduction Test、既有测试和相关流程测试。
10. 对 UI 或运行时问题再次执行真实操作、截图、Console/Network 检查。
11. 审查 Diff 是否包含无关改动，记录证据和剩余风险。
12. Commit 并通过 PR Review；不要把猜测、临时日志或未验证方案当作完成。

## 24. 完整 Long-task SOP

1. 在开始时建立任务 Spec、Task Plan、代码地图、验收标准和当前 Git 基线。
2. 每完成一个可验证单元就记录状态、决策、测试和 Diff。
3. 发现新问题时更新文档；区分事实、假设、决策和建议。
4. 当会话变长或出现 Context Drift 时停止继续堆对话。
5. 写完整 Handoff，包含目标、Source of Truth、代码位置、决策链、尝试、事实、假设、风险、Git 和测试状态。
6. 保存清晰的当前 Git 状态。
7. 开启新会话；新 Agent 先读相关真实代码和测试，再读 Handoff，并回到代码核验。
8. 新 Agent 输出自己的理解并完成认知对齐后，再继续 Plan、Implement 或 Debug。

## 25. 完整 Sub-agent SOP

1. 明确子代理角色：执行、调查、Debug、Review 或测试。
2. 定义 Objective、输出格式、允许范围、禁止范围和验证标准。
3. 按角色提供上下文：
   - 执行型：提供已批准方案和充分决策信息；
   - 调查/Debug/Review 型：只提供最小充分事实、代码范围和约束，隔离主 Agent 观点。
4. 要求子代理先读真实代码、测试、Diff 和相关文档，先写自己的系统理解。
5. 调查型子代理先独立分析，再接收主 Agent 的怀疑或方案。
6. 收集其结论、证据、未知点和替代解释。
7. 主 Agent 与子代理结论进行 Compare，并用实验或测试裁决。
8. 只有执行方案已经对齐、范围明确且验收标准可验证时，才授权修改。
9. 子代理返回 Diff、测试、运行结果、截图和剩余风险。
10. 主 Agent 独立 Review；子代理的结论不能替代最终验收。

## 26. 最终执行检查

在结束前问自己：

- 我是否做了必要调研，而不是从零猜测？
- 关键歧义是否已经澄清并写入文档？
- 我是否先读了真实代码并追踪了执行链？
- File/Call/Data/Impact Map 是否足以解释本次变化？
- 方案是否被理解、比较并批准？
- Plan 是否提前定义了可观察的 Done？
- 修改是否局限在允许范围？
- Debug 是否有证据支持 Root Cause？
- 是否完成了 Correctness、Acceptance、Regression 和适用的 Visual Verification？
- 是否用截图、浏览器、日志或测试结果证明真实行为？
- Git Diff、Commit、PR 和文档是否足以让别人复查和接手？
- 是否仍有未验证项、未知风险或需要用户决定的地方？

如果任一关键问题答不上来，不要用“应该没问题”结束；报告缺口并继续验证或请求澄清。

> 最重要的原则：不要许愿式编程。不要期待 Agent 一次猜对所有事情；把工作拆成能被理解、观察、验证、审查、纠正和回滚的工程步骤。
