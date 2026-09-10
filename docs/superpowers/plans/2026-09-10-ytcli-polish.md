# ytcli Polish Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans or implement inline. Steps use checkbox syntax.

**Goal:** shuffle pendientes, playlist YouTube, loop/unloop per spec `2026-09-10-ytcli-polish-design.md`.

**Architecture:** Extiende queue/youtube/watch/commands/state; sin TUI ni Spotify.

**Tech Stack:** Rust existente + yt-dlp flat-playlist.

## Global Constraints

- shuffle solo `queue[current+1..]`
- playlist encola; `--play` reemplaza + reproduce; `-n` default 100
- loop_current; watch reinicia; clear en next, prev-f, prev GoTo, play, playlist --play, stop
- prev SeekZero mantiene loop
- Mensajes en español

---

### Task 1: state + shuffle + playlist parse + loop watch + CLI/commands + README

Implementación única coherente (feature pequeña). Ver spec para comportamiento exacto.
