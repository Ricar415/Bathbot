pub struct DbRenderOptions {
    pub official_skin_name: String,
    pub official_skin_display_name: String,
    pub custom_skin_id: Option<i32>,
    pub custom_skin_display_name: Option<String>,
    pub global_volume: i16,
    pub music_volume: i16,
    pub hitsound_volume: i16,
    pub show_hit_error_meter: bool,
    pub show_unstable_rate: bool,
    pub show_score: bool,
    pub show_hp_bar: bool,
    pub show_combo_counter: bool,
    pub show_pp_counter: bool,
    pub show_key_overlay: bool,
    pub show_scoreboard: bool,
    pub show_borders: bool,
    pub show_mods: bool,
    pub show_result_screen: bool,
    pub use_skin_cursor: bool,
    pub use_skin_colors: bool,
    pub use_skin_hitsounds: bool,
    pub use_beatmap_colors: bool,
    pub cursor_scale_to_cs: bool,
    pub cursor_rainbow: bool,
    pub cursor_trail_glow: bool,
    pub draw_follow_points: bool,
    pub draw_combo_numbers: bool,
    pub cursor_size: f32,
    pub cursor_trail: bool,
    pub beat_scaling: bool,
    pub slider_merge: bool,
    pub objects_rainbow: bool,
    pub flash_objects: bool,
    pub use_slider_hitcircle_color: bool,
    pub seizure_warning: bool,
    pub load_storyboard: bool,
    pub load_video: bool,
    pub intro_bg_dim: i16,
    pub ingame_bg_dim: i16,
    pub break_bg_dim: i16,
    pub bg_parallax: bool,
    pub show_danser_logo: bool,
    pub skip_intro: bool,
    pub cursor_ripples: bool,
    pub slider_snaking_in: bool,
    pub slider_snaking_out: bool,
    pub show_hit_counter: bool,
    pub show_avatars_on_scoreboard: bool,
    pub show_aim_error_meter: bool,
    pub play_nightcore_samples: bool,
}
