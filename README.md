# PiCamWebStream

Easily deploy a Raspberry Pi camera web stream with a Rust backend and a modern Svelte + Tailwind frontend. This repository contains everything you need to run locally or with Docker Compose.

## Project layout

```
.
├── backend/        # Rust (Axum) service that captures MJPEG frames and exposes APIs
├── frontend/       # Svelte + Tailwind single-page app that displays the stream
├── docker-compose.yml
└── README.md
```

### Backend

-   Language: Rust (edition 2021)
-   Framework: [`axum`](https://github.com/tokio-rs/axum)
-   Responsibilities:
    -   Provide `/stream` endpoint streaming MJPEG data
    -   Serve `/config` JSON describing capture settings
    -   Health check via `/health`
    -   Uses a V4L2 camera on Linux by default (e.g. `/dev/video0`), falling back to the mock generator when unavailable.

Environment variables:

| Variable        | Default                | Description                                               |
| --------------- | ---------------------- | --------------------------------------------------------- |
| `BACKEND_HOST`  | `0.0.0.0`              | Address to bind the HTTP server                           |
| `BACKEND_PORT`  | `8080`                 | HTTP port                                                 |
| `FRAME_RATE`    | `12`                   | Target frames per second (1-60)                           |
| `FRAME_WIDTH`   | `1280`                 | Stream width                                              |
| `FRAME_HEIGHT`  | `720`                  | Stream height                                             |
| `CAMERA_DEVICE` | `/dev/video0` on Linux | V4L2 device path; unset or empty to force the mock camera |

### Frontend

-   Framework: [Svelte](https://svelte.dev/) with TypeScript
-   Styling: [Tailwind CSS](https://tailwindcss.com/)
-   Fetches backend config + health status and displays the MJPEG stream.

To point to a different backend, set `VITE_BACKEND_URL` (defaults to `http://localhost:8080` in development, `http://backend:8080` inside docker-compose).

## Development

### Backend

```bash
cd backend
cargo run
```

### Frontend

```bash
cd frontend
npm install
npm run dev
```

Access the app at http://localhost:5173 (Vite dev server). The dev server proxies `/stream`, `/config`, and `/health` to the Rust service.

## Docker

Build and run both services with Docker Compose:

```bash
docker compose up --build
```

-   Frontend available at http://localhost:3000
-   Backend API (and mock MJPEG stream) at http://localhost:8080

The frontend container now serves the built app using Vite's preview server (listening on port 4173 inside the container and forwarded to port 3000 on your host).

### Individual images

```bash
# Backend
cd backend
docker build -t picam-backend .

# Frontend
cd frontend
npm install
npm run build
docker build -t picam-frontend .
# Runs on port 4173 inside the container
docker run --rm -p 3000:4173 picam-frontend
```

## Next steps

-   Extend camera support beyond V4L2 if needed (e.g., libcamera bindings or remote streams).
-   Add authentication for stream access.
-   Introduce persistent configuration storage if needed.
-   Expand frontend controls (e.g., frame rate selection, snapshots).

## License

This project is released under the [MIT License](LICENSE).
