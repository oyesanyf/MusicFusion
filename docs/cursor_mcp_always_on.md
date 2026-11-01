# MCP “always on” in Cursor (what works vs. what doesn’t)

Here’s what Cursor actually supports, and how to get as close as possible to “MCP for everything I type.”

## What Cursor does not give you

There is no documented switch that means: “on every message, always run all MCP servers/tools with no @ and no filtering.”

Docs describe the agent using MCP from **Available tools** when they’re relevant, not “always invoke every server on every prompt.”

So you can make MCP always available and easy to use, but not force literal use on every single line you type.

## How to get MCP “always on” in practice

### 1. Enable your servers globally

Put servers in `%USERPROFILE%\.cursor\mcp.json` so they apply to all projects (you can still add/override in a project’s `.cursor/mcp.json`; project wins on name conflicts).

That way you’re not depending on `@` to “install” the server for the repo.

### 2. Turn them on in Settings

**Cursor Settings → Tools & MCP** (sometimes listed under **Features → Model Context Protocol** in help).

Enable the servers you want. Disabled servers don’t load, so they can’t be used.

### 3. Use Agent (not Ask) for MCP tool use

MCP is wired to agent-style chat that can call tools. Ask mode is read-only Q&A and isn’t the right mode if you want MCP tools in the loop.

### 4. Reduce friction in the prompt: Auto-run

If the blocker is approval prompts before each MCP call, look for **Auto-run** / permissions for MCP (and related options in [Cursor MCP docs](https://cursor.com/docs/mcp)). That makes tool use smoother; it still doesn’t mean “run MCP on every message,” it means the agent can call tools when it decides to with fewer clicks.

### 5. Optional: pin tools in chat

In the chat panel you can toggle which tools are in play. Leave your gateway (or other MCP tools) on so the model doesn’t “forget” they exist for that thread.

## About @

`@` is mainly for attaching servers, files, or docs to that conversation. You don’t need `@` every time if the server is enabled in Settings and loaded. Some UIs still feel easier if you `@` an MCP server once to pin it to the thread—that’s UI preference, not the only way servers can work.

## Summary

Put MCP in `~/.cursor/mcp.json` (global) or project `.cursor/mcp.json`, enable servers under **Tools & MCP**, chat in **Agent**, and use **auto-run** if you want fewer interruptions. That is the supported way to have MCP ready for everything you type; there is no official “run all MCP on every keystroke” mode.
