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

### General Configuration

You can configure the project using environment variables. When using Docker Compose, you can create a `.env` file by copying the example:

```bash
cp .env.example .env
```

| Variable        | Default                | Description                                               |
| --------------- | ---------------------- | --------------------------------------------------------- |
| `PORT`          | `3000`                 | Public port to access the web interface                   |
| `BACKEND_HOST`  | `0.0.0.0`              | Address for the backend to bind to                        |
| `BACKEND_PORT`  | `8080`                 | Internal port for the backend                             |
| `FRAME_RATE`      | `30`                   | Target frames per second (1-60)                           |
| `FRAME_WIDTH`     | `3840`                 | Stream width                                              |
| `FRAME_HEIGHT`    | `2160`                 | Stream height                                             |
| `CAMERA_DEVICE`   | `/dev/video0` on Linux | V4L2 device path; unset or empty to force the mock camera |
| `STREAM_USER`     | (unset)                | Username for Basic Auth (optional)                        |
| `STREAM_PASSWORD` | (unset)                | Password for Basic Auth (optional)                        |

## Home Assistant Integration

You can easily add this camera to your Home Assistant dashboard using the **Generic Camera** or **MJPEG** integration. Use the public `PORT` (default 3000) for access.

### Option 1: Generic IP Camera (Preferred)
In your Home Assistant `configuration.yaml`:

```yaml
camera:
  - platform: generic
    name: "Pi Camera"
    still_image_url: http://<PI_IP_ADDRESS>:3000/snapshot
    stream_source: http://<PI_IP_ADDRESS>:3000/stream
    authentication: basic
    username: "your_user" # if configured
    password: "your_password" # if configured
```

### Option 2: MJPEG IP Camera
```yaml
camera:
  - platform: mjpeg
    name: "Pi Camera Stream"
    mjpeg_url: http://<PI_IP_ADDRESS>:3000/stream
```

### Frontend

-   Framework: [Svelte](https://svelte.dev/) with TypeScript
-   Styling: [Tailwind CSS](https://tailwindcss.com/)
-   Fetches backend config + health status and displays the MJPEG stream.

To point to a different backend, set `VITE_BACKEND_URL`. If unset, the frontend will automatically connect to the backend (via the Nginx proxy) using the same IP address/hostname used to access the website.

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
-   Backend API (directly) at http://localhost:8080

The frontend container serves the built app using Nginx (listening on port 80 inside the container and forwarded to port 3000 on your host by default).

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
# Runs on port 80 inside the container
docker run --rm -p 3000:80 picam-frontend
```

## Next steps

-   Extend camera support beyond V4L2 if needed (e.g., libcamera bindings or remote streams).
-   Add authentication for stream access.
-   Introduce persistent configuration storage if needed.
-   Expand frontend controls (e.g., frame rate selection, snapshots).

## License

This project is released under the [MIT License](LICENSE).
