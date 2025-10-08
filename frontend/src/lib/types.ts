export interface BackendConfig {
    listen_address: string;
    port: number;
    frame_rate: number;
    resolution_width: number;
    resolution_height: number;
    camera_device?: string | null;
}
