# Claude Code notes for this repository

- Prefer **Sonnet subagents** for delegated work (implementation, research, validation runs); keep the main loop for review, delegation, and high-level decisions. Escalate a subagent to a stronger model only when a task has repeatedly failed or is unusually judgment-heavy.
- Never write feedback about the default "commit nag" stop hook in the chat. When the hook flags uncommitted or unpushed changes that belong to an in-flight agent or are otherwise intentionally staged for a later commit, just continue silently; only act on it when the changes are genuinely finished work that should be committed and pushed.
- Whenever a preview deploy is ready (Pages publish succeeded), always post the site link in the chat: https://mentalgear.github.io/dioxus-components/ (with the main commit it was built from).
- Delegate debugging to Sonnet subagents too (reproduce, bisect, root-cause, propose the construction); the main loop reviews the evidence and decides. Do not run long hands-on debugging loops in the main conversation.
