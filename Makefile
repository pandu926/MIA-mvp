.PHONY: up down build logs migrate db-shell redis-cli backend-shell test-backend build-frontend help

help:
	@echo "MIA — AI-native investigation MVP"
	@echo ""
	@echo "Commands:"
	@echo "  make up            Start the full stack in Docker"
	@echo "  make build         Build all Docker images"
	@echo "  make down          Stop all services"
	@echo "  make migrate       Run database migrations"
	@echo "  make db-shell      Open PostgreSQL shell"
	@echo "  make redis-cli     Open Redis CLI"
	@echo "  make logs          Follow logs for all services"
	@echo "  make backend-shell Open shell in backend container"
	@echo "  make test-backend  Run backend tests"
	@echo "  make build-frontend Build the frontend locally"

up:
	cp -n .env.example .env 2>/dev/null || true
	docker compose up -d --build

down:
	docker compose down

build:
	docker compose build

migrate:
	docker compose exec backend sqlx migrate run

db-shell:
	docker compose exec postgres psql -U mia -d mia_db

redis-cli:
	docker compose exec redis redis-cli

logs:
	docker compose logs -f

backend-shell:
	docker compose exec backend bash

test-backend:
	cd backend && cargo test

build-frontend:
	cd frontend && npm run build
