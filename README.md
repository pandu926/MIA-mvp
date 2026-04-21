# MIA MVP

MIA is an AI-native investigation and monitoring desk for chaotic Four.Meme launches on BNB Chain.

This open-source MVP focuses on one problem: fast launches create too much fragmented context for humans to investigate consistently. MIA turns that into a run-based workflow with AI scoring, monitoring continuity, deep research attachment, and evidence history.

## What the MVP does

- discovers live token candidates
- opens an investigation workspace for a single token
- gates AI scoring by activity threshold
- attaches Deep Research when more evidence is justified
- keeps run history, watchlists, missions, and monitoring state
- exposes Ask MIA as a grounded investigation layer

## Stack

- Backend: Rust, Axum, Tokio, SQLx
- Frontend: Next.js 15, React 19, TypeScript
- Data: PostgreSQL, Redis
- Chain: BNB Chain / Four.Meme
- Enrichment: Moralis + optional Deep Research providers
- LLM: any OpenAI-compatible endpoint

## Quick start

1. Copy the env template:

```bash
cp .env.example .env
```

2. Fill the required values in `.env`:

- `BNB_RPC_WS_URL`
- `LLM_API_URL`
- `LLM_API_KEY`
- `MORALIS_API_KEY`

3. Start the stack:

```bash
docker compose up -d --build
```

4. Open:

```text
http://localhost:3313
```

## Required environment variables

Minimum required for the MVP to function well:

- `BNB_RPC_WS_URL`: BNB Chain websocket RPC
- `LLM_API_URL`: your OpenAI-compatible inference endpoint
- `LLM_API_KEY`: key for that endpoint
- `MORALIS_API_KEY`: holder and enrichment data

Optional but supported:

- `HEURIST_API_KEY`
- `X402_*`
- `TELEGRAM_*`

## Local commands

Start everything:

```bash
make up
```

Stop everything:

```bash
make down
```

Backend tests:

```bash
make test-backend
```

Frontend production build:

```bash
make build-frontend
```

## Main surfaces

- `/app` — live discovery surface
- `/mia` — investigation workspace
- `/mia/runs` — run inbox and continuity
- `/mia/watchlist` — persistent monitoring layer
- `/mia/missions` — grouped operator goals
- `/backtesting` — proof and replay support

## Product direction

This repository is the MVP wedge for a larger goal: an AI-native research and monitoring firm where discovery, investigation, escalation, deep research, and eventually execution can be coordinated by cooperating AI systems with human oversight.

The MVP is intentionally narrower:

- no unrestricted autonomous execution
- no self-modifying agents in production
- no promise of zero-human operations

It is a controlled operating surface, not automation theater.

## Repository scope

This public repository is intentionally kept lean:

- runnable app code only
- no private wallets
- no internal planning docs
- no local agent configs
- no cached build artifacts

## License

No license has been attached yet. Add one before broader public reuse if you want to permit redistribution or modification explicitly.
