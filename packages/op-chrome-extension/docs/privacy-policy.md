# Privacy policy — OpenPencil (Chrome extension)

_Last updated: 2026-08-06. Applies to the Chrome extension **OpenPencil**,
version 0.8.2 and later (listed as "OpenPencil Web Capture" before 0.8.3)._

This document is the source text for the privacy policy URL required by the
Chrome Web Store listing. Publish it verbatim; if the extension's behaviour
changes, change this file in the same commit.

---

## English

### The short version

The extension reads a web page only when you press one of its buttons, and
sends what it read only to a destination you configured: an OpenPencil running
on your own computer, or — if you chose to sign in — your own OpenPencil
account. Nothing is sent anywhere else. There is no analytics, no telemetry,
no advertising, no third-party service of any kind.

### What is collected, and when

**Page content — only on your action.** Pressing **Capture full page** or
**Capture element** runs the OpenPencil DOM extractor in the tab that is
currently open. It reads the rendered document: element geometry, computed
styles, text content, and images (which are rasterized into the snapshot).
That means a capture can contain anything visible on the page at that moment,
including text you typed into a form and content behind a login. It is
produced in memory, in your browser, on your press, and it is never captured
in the background, on a schedule, or on pages you have not acted on.

**Nothing is read from tabs you do not act on.** The extension holds no
`<all_urls>` permission. It uses `activeTab`, which Chrome grants for one tab
at the moment you click the toolbar button and revokes afterwards.

**Browsing history is not collected.** The extension does not observe
navigation, does not enumerate your tabs, and keeps no record of the pages you
have captured.

### Where a capture goes

You choose one of three destinations, every one of which you configure:

1. **An OpenPencil on this computer** (the default). The snapshot is POSTed to
   the loopback address in the popup's settings — `127.0.0.1:3100` unless you
   changed it. The extension can only address loopback: the destination is
   validated against a whitelist of `127.0.0.1`, `[::1]` and `localhost`, and
   the manifest grants no other host for this purpose. The data never leaves
   your machine.
2. **A file on this computer.** **Download .op** writes a ready-to-open `.op`
   document to your downloads folder and sends it nowhere.
3. **Your OpenPencil account** — only if you sign in **and** select it. When
   you are signed in, a **Send to** setting appears with two destinations, and
   it is set to the OpenPencil on this computer unless you change it. If you
   select your account, the capture is uploaded to the OpenPencil Hub you are
   signed into — `https://op.zseven.cn` or `https://op.zseven.tech`, whichever
   region you chose — and to no one else. The upload carries the page title
   (as the snapshot's name), the page's address, the time of capture, and the
   snapshot itself. It is filed in your own account's inbox, where only you can
   read it; you can delete it from the account page at any time, and it is
   deleted automatically after 30 days. If your session has expired at the
   moment you press the button, the capture goes to destination 1 instead — it
   is never uploaded on an expired session.

### Signing in is optional

Everything above works signed out, and always will. If you do sign in:

- The extension opens the OpenPencil Hub's own sign-in page in a normal
  browser tab. Your credentials are entered on that site, never in the
  extension. The extension never sees your password, an authorization code,
  or any long-lived token.
- The Hub sets its own session cookie on its own domain. That cookie is
  `HttpOnly`, so the extension cannot read it, and the extension does not
  request the `cookies` permission that would let it try.
- The extension asks the Hub `GET /api/v1/session` to learn who is signed in,
  and stores locally only what it paints in the popup header: your display
  name, your avatar's URL, your account's opaque user id, and which region you
  selected. Your email address is deliberately not requested from that reply,
  not stored, and not displayed.
- The session's CSRF token is held in memory only, for as long as the popup is
  open (or one background task runs). It is required by the Hub to accept an
  upload or a sign-out, and it is never written to storage.
- Signing out clears that local copy immediately.

The two hubs the extension may contact are fixed at build time and listed in
the manifest: `https://op.zseven.cn` (China) and `https://op.zseven.tech`
(Global). It cannot be pointed at any other public host — including for
uploads.

### What is stored on your device

In `chrome.storage.local`, on your machine only:

| Key              | What                                                   |
| ---------------- | ------------------------------------------------------ |
| `endpoint`       | The loopback address you typed.                        |
| `uiLocale`       | The popup's language.                                  |
| `lastAction`     | Whether your last delivery was "send" or "download".   |
| `pickResult`     | The outcome of one element capture, until you read it. |
| `hubRegion`      | `cn` or `global`.                                      |
| `deliveryTarget` | Which destination you selected.                        |
| `account`        | Display name, avatar URL, user id, region.             |

No session token, no CSRF token and no password is ever written to storage.
Uninstalling the extension removes all of it.

### Permissions, and why each one exists

| Permission                 | Why                                                                                      |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| `activeTab`                | Read the page you pressed the button on — one tab, one action, revoked afterwards.       |
| `scripting`                | Inject the extractor, the transfer harness and the element-picker overlay into that tab. |
| `downloads`                | Save the `.op` document when you press **Download .op**.                                 |
| `storage`                  | Remember the settings in the table above.                                                |
| `http://127.0.0.1/*`       | Deliver the snapshot to the OpenPencil running on your computer.                         |
| `https://op.zseven.cn/*`   | Ask the China hub who is signed in, open its sign-in page, and upload a capture when you selected your account as the destination. Only used if you sign in. |
| `https://op.zseven.tech/*` | The same, for the Global hub.                                                            |

There is deliberately **no** `<all_urls>`, no `cookies`, no `history`, no
`tabs`, and no `webRequest` permission.

### What is not done

No analytics or telemetry of any kind. No advertising or advertising
identifiers. No user tracking. No selling or transferring data to third
parties. No use of your data to train models. No remote code: everything the
extension runs ships inside the package, as the Manifest V3 rules require.

### Contact

Questions, or a report: <https://github.com/ZSeven-W/openpencil/issues>.

---

## 中文

### 一句话版本

只有当你按下扩展的按钮时，它才会读取网页；读到的内容只会发往你自己配置的目的
地：运行在你自己电脑上的 OpenPencil，或者（如果你选择登录）你自己的 OpenPencil
账号。除此之外不会发往任何地方。没有任何统计、遥测、广告或第三方服务。

### 收集什么，何时收集

**网页内容 —— 仅在你操作时。** 按下 **捕获整页** 或 **捕获元素** 时，扩展会在
当前标签页中运行 OpenPencil 的 DOM 提取器，读取渲染后的文档：元素几何、计算样
式、文本内容和图片（图片会被栅格化进快照）。因此一次捕获可能包含该时刻页面上
一切可见的内容，包括你填入表单的文字以及登录后才能看到的内容。它在你的浏览器
内存中生成，由你的按键触发；扩展不会在后台、按计划或在你没有操作过的页面上进
行捕获。

**不会读取你没有操作的标签页。** 扩展没有 `<all_urls>` 权限，它使用
`activeTab`：Chrome 只在你点击工具栏按钮的那一刻为该标签页授权，之后即收回。

**不收集浏览记录。** 扩展不监听导航、不枚举标签页，也不保存你捕获过哪些页面。

### 捕获的内容去往哪里

三个目的地，全部由你配置：

1. **本机的 OpenPencil**（默认）。快照会 POST 到弹窗设置中的回环地址，未修改时
   为 `127.0.0.1:3100`。扩展只能寻址回环：目标地址会经过 `127.0.0.1`、`[::1]`、
   `localhost` 的白名单校验，清单文件也没有为此授予其他主机。数据不会离开你的
   电脑。
2. **本机的一个文件。** **下载 .op** 会把可直接打开的 `.op` 文档写入下载文件夹，不发往任何地方。
3. **你的 OpenPencil 账号** —— 仅在你登录**并且**主动选择它时。登录后设置里会
   出现"发送到"一项，默认仍是本机的 OpenPencil，除非你改动它。选择账号后，捕获
   会上传到你登录的那个 OpenPencil Hub（`https://op.zseven.cn` 或
   `https://op.zseven.tech`，取决于你选择的区域），不会发往任何其他地方。上传内
   容包括页面标题（作为快照名称）、页面地址、捕获时间以及快照本身。它保存在你自
   己账号的收件箱中，只有你能读取；你可以随时在账号页面删除，30 天后也会自动删
   除。如果按下按钮时会话已过期，捕获会改为送往第 1 个目的地 —— 绝不会用一个已
   过期的会话上传。

### 登录是可选的

上面所有功能在未登录时都可用，将来也一样。如果你选择登录：

- 扩展会在普通浏览器标签页中打开 OpenPencil Hub 自己的登录页。你的凭据输入在那
  个网站上，而不是在扩展里。扩展看不到你的密码、授权码或任何长期令牌。
- Hub 在它自己的域名下设置会话 Cookie。该 Cookie 带 `HttpOnly`，扩展读不到；扩
  展也没有申请可以尝试读取它的 `cookies` 权限。
- 扩展通过 `GET /api/v1/session` 询问当前登录者是谁，本地只保存弹窗标题栏要画出
  来的内容：显示名、头像 URL、账号的不透明用户 id，以及你选择的区域。你的邮箱地
  址被刻意排除：不取用、不保存、不显示。
- 会话的 CSRF 令牌只保存在内存中，仅在弹窗打开期间（或一次后台任务运行期间）存
  在。Hub 要求它才会接受上传或退出登录请求，它绝不会写入存储。
- 退出登录会立即清除这份本地副本。

扩展可以联系的两个 Hub 在构建时固定，并列在清单文件中：`https://op.zseven.cn`
（中国）与 `https://op.zseven.tech`（全球）。它无法被指向任何其他公网主机 ——
上传同样如此。

### 在你设备上保存了什么

仅保存在你本机的 `chrome.storage.local` 中：

| 键               | 内容                              |
| ---------------- | --------------------------------- |
| `endpoint`       | 你填写的回环地址。                |
| `uiLocale`       | 弹窗语言。                        |
| `lastAction`     | 上次是"发送"还是"下载"。          |
| `pickResult`     | 一次元素捕获的结果，读过即清。    |
| `hubRegion`      | `cn` 或 `global`。                |
| `deliveryTarget` | 你选择的目的地。                  |
| `account`        | 显示名、头像 URL、用户 id、区域。 |

会话令牌、CSRF 令牌和密码都不会写入存储。卸载扩展会一并删除以上全部内容。

### 各项权限及其理由

| 权限                       | 理由                                                        |
| -------------------------- | ----------------------------------------------------------- |
| `activeTab`                | 读取你按下按钮的那个页面：一个标签页、一次操作，随后收回。  |
| `scripting`                | 向该标签页注入提取器、传输桥和元素选取浮层。                |
| `downloads`                | 你按下 **下载 .op** 时保存 `.op` 文档。                     |
| `storage`                  | 记住上表中的设置。                                          |
| `http://127.0.0.1/*`       | 把快照送到你电脑上运行的 OpenPencil。                       |
| `https://op.zseven.cn/*`   | 询问中国区 Hub 当前登录者、打开其登录页，并在你选择账号为目的地时上传捕获。仅在登录时使用。 |
| `https://op.zseven.tech/*` | 同上，用于全球区 Hub。                                      |

扩展刻意**没有** `<all_urls>`、`cookies`、`history`、`tabs`、`webRequest` 权限。

### 不做什么

不做任何统计或遥测。没有广告和广告标识符。不做用户追踪。不向第三方出售或转移数
据。不使用你的数据训练模型。不加载远程代码：扩展运行的一切都随安装包一起交付，
这也是 Manifest V3 的要求。

### 联系方式

问题与反馈：<https://github.com/ZSeven-W/openpencil/issues>。
