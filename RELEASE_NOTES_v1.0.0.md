# Physics Clawd Pet v1.0.0

The first stable release. Clawd now has a calendar and to-do assistant, reacts to Claude Code, a second character, and full Chinese, Japanese and English UI.

## Highlights

- **Assistant window:** to-dos with due times, iCal subscriptions (Google, Outlook, Feishu, DingTalk, and others) or local `.ics` files, and a reminder scheduler. Reminders appear as a bubble above Clawd, and it waits for you to respond.
- **Claude Code integration (optional, off by default):** Clawd mirrors Claude Code's activity, approves permission requests from its bubble, and drops a mini Clawd for each subagent.
- **Dots:** a second character, a smooth, furry blue fluffball in a beret. Switch from the tray menu.
- **Localization:** tray menu, Assistant window and speech follow the Windows display language: Chinese, Japanese or English. Japanese Clawd speaks Osaka dialect.

## New

- Calendar and iCal subscription, with a pre-event run to the middle of the screen.
- To-dos with due-time reminders, completable directly from the bubble.
- Claude Code reactions: thinking, working, compacting, finishing a turn, errors, and subagents. Each can be turned off separately.
- Permission bubbles: Allow / Deny from the pet while Claude Code is waiting for approval.
- Dots skin (`skins/dots/`), rendered at full resolution with procedural fur.
- Chinese, Japanese and English interface strings.

## Changed

- Version set to 1.0.0 across the app, Tauri config and Cargo manifest.
- The installer ships in English, Japanese and Simplified Chinese.

## Upgrade notes

- If you already use the Claude Code integration, the app updates the installed hook paths on launch. If you moved the install folder, press **Reconnect** in the Assistant window.
- The installer's uninstall step removes the Clawd hooks from `~/.claude/settings.json`. Your own hooks are not changed.

## Known issues

- The tuning panel is Chinese-only for now.
- Windows 10 and 11 only.

## Download

Get the installer from the assets below. It needs WebView2, which Windows 11 includes.

---

# Physics Clawd Pet v1.0.0（中文）

第一个正式版。Clawd 现在有了日历与待办小助手、能跟 Claude Code 联动、第二个形象 Dots，以及完整的中文、日文、英文界面。

## 亮点

- **小助手窗口：** 带截止时间的待办、iCal 订阅（Google、Outlook、飞书、钉钉等）或本地 `.ics` 文件，以及提醒调度。提醒会在 Clawd 头顶冒气泡，它会等你回应。
- **Claude Code 联动（可选，默认关闭）：** Clawd 跟着 Claude Code 的状态动，在气泡上批准权限请求，每个子代理派出一只迷你 Clawd。
- **Dots：** 第二个形象，平滑、毛茸茸的戴贝雷帽蓝色毛球。在托盘菜单里切换。
- **多语言：** 托盘菜单、小助手窗口和台词跟随 Windows 显示语言：中文、日文或英文。日文版 Clawd 说关西腔。

## 新增

- 日历与 iCal 订阅，日程开始前 Clawd 跑到屏幕中间举着日历页。
- 带截止时间提醒的待办，可直接在气泡上勾掉。
- Claude Code 反应：思考、干活、压缩上下文、一轮完成、出错、子代理。每种都可单独关闭。
- 权限气泡：Claude Code 等待批准时，可以直接在宠物上点允许 / 拒绝。
- Dots 皮肤（`skins/dots/`），按实际分辨率绘制，毛发程序生成。
- 中文、日文、英文界面文字。

## 变更

- 版本号统一为 1.0.0（应用、Tauri 配置、Cargo 清单）。
- 安装程序提供英文、日文、简体中文。

## 升级说明

- 如果已经开启过 Claude Code 联动，程序启动时会自动更新 hooks 里的程序路径。如果挪过安装目录，请在小助手窗口点 **重新开启**。
- 卸载时会从 `~/.claude/settings.json` 删除 Clawd 的 hooks，你自己的 hooks 不受影响。

## 已知问题

- 调教面板目前只有中文。
- 仅支持 Windows 10 / 11。

## 下载

在下方 Assets 中获取安装包。需要 WebView2（Windows 11 自带）。
