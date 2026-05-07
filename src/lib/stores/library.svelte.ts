import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from '@tauri-apps/api/core';

export interface Album {
    id: number;
    title: string;
    artist: string | null;
    cover_art_path: string | null;
}

export interface LocalTrack {
    id: number;
    title: string;
    artist: string | null;
    album_id: number | null;
    track_number: number | null;
    file_path: string;
}

export interface Playlist {
    id: number;
    name: string;
}

export class LibraryStore {
    albums = $state<Album[]>([]);
    playlists = $state<Playlist[]>([]);
    isScanning = $state(false);

    async fetchAlbums() {
        try {
            this.albums = await invoke("get_albums");
        } catch (e) {
            console.error(e);
        }
    }

    async fetchPlaylists() {
        try {
            this.playlists = await invoke("get_playlists");
        } catch (e) {
            console.error(e);
        }
    }

    async scanDirectory(path: string) {
        if (!path) return;
        this.isScanning = true;
        try {
            await invoke("scan_local_directory", { path });
            await this.fetchAlbums();
        } catch (e) {
            console.error("Scan error:", e);
        } finally {
            this.isScanning = false;
        }
    }

    async getAlbumTracks(albumId: number): Promise<LocalTrack[]> {
        return await invoke("get_album_tracks", { albumId });
    }

    async getPlaylistTracks(playlistId: number): Promise<LocalTrack[]> {
        return await invoke("get_playlist_tracks", { playlistId });
    }
    
    async createPlaylist(name: string) {
        await invoke("create_playlist", { name });
        await this.fetchPlaylists();
    }
    
    async addToPlaylist(playlistId: number, trackId: number) {
        await invoke("add_to_playlist", { playlistId, trackId });
    }

    async getArtworkUrl(trackId: number, filePath: string): Promise<string | null> {
        try {
            const cachedPath = await invoke<string | null>("extract_and_cache_artwork", { trackId, filePath });
            if (cachedPath) {
                return convertFileSrc(cachedPath);
            }
        } catch (e) {
            console.error(e);
        }
        return null;
    }
    
    async clearLibrary() {
        try {
            await invoke("clear_local_library");
            this.albums = [];
            this.playlists = [];
        } catch(e) {
            console.error(e);
        }
    }
}

export const libraryStore = new LibraryStore();
