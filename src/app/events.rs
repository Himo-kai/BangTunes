// SPDX-License-Identifier: MIT
// Copyright (c) 2024 BangTunes Contributors

/// Application tab selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppTab {
    Library,
    Playlists,
    MetadataEditor,
    Settings,
}

/// Which metadata field is being edited in the Metadata Editor tab
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    None,
    Title,
    Artist,
}

/// Playback repeat cycle
#[derive(Debug, Clone, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

/// Every event the TUI can process (input, internal ticks, audio notifications)
#[derive(Debug, Clone)]
pub enum InteractiveEvent {
    Quit,
    Tick,
    Play,
    TogglePlayPause,
    NextTrack,
    PreviousTrack,
    Up,
    Down,
    VolumeUp,
    VolumeDown,
    ToggleRepeat,
    ToggleShuffle,
    ToggleAutoplay,
    // Tab navigation
    SwitchToLibrary,
    SwitchToPlaylists,
    SwitchToMetadataEditor,
    SwitchToSettings,
    // Metadata editor
    EditTitle,
    EditArtist,
    SaveMetadata,
    CancelEdit,
    ApplySuggestion,
    #[allow(dead_code)] // Used in metadata editor event handling
    ResetToOriginal,
    BulkApplySuggestions,
    ClearMetadata,
    // Generic UI
    ShowHelp,
    Input(char),
    Backspace,
    // Search
    EnterSearch,
    ExitSearch,       // Esc — clear query, restore full library
    ConfirmSearch,    // Enter — lock in results, exit input overlay, keep filtered view
    SearchInput(char),
    SearchBackspace,
    // Playlists
    StartPlaylistCreation,
    DeletePlaylist,
    RenamePlaylist,
    AddToPlaylist,
    RemoveFromPlaylist,
    LoadPlaylist,
    TogglePlaylistExpansion,
    PlaylistInput(char),
    PlaylistBackspace,
    ConfirmPlaylistCreation,
    CancelPlaylistCreation,
    // Playlist selector overlay
    SelectPlaylistFromSelector,
    CancelPlaylistSelector,
    // Queue management
    QueuePlayNext,           // 'e'  — insert at front
    QueueAddToEnd,           // 'E'  — append to end
    QueueClear,              // 'C'  — clear entire queue
    ToggleQueue,             // 'Q'  — toggle queue overlay (Library tab)
    QueueRemove,             // 'x'  — remove selected track (overlay open)
    LoadPlaylistToQueue,     // 'Q'  — load playlist into queue (Playlists tab)
    ConfirmQueueReplace,     // 'Y'  — confirm replacing queue with playlist
    CancelQueueReplace,      // 'N'/Esc — cancel queue replacement
    // Favorites
    ToggleFavorite,          // 'f'  — toggle ⭐ (boosts shuffle weight)
    // Recovery
    ForceRedraw,             // Ctrl+L — full terminal redraw to clear glitches
}
