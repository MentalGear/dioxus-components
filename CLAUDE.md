# Claude Code notes for this repository

- Never write feedback about the default "commit nag" stop hook in the chat. When the hook flags uncommitted or unpushed changes that belong to an in-flight agent or are otherwise intentionally staged for a later commit, just continue silently; only act on it when the changes are genuinely finished work that should be committed and pushed.
