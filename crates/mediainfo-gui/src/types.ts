export interface GeneralTrack {
  format: string;
  format_version?: string;
  format_profile?: string;
  codec_id?: string;
  file_size: number;
  file_path?: string;
  file_name?: string;
  duration_ms?: number;
  overall_bitrate?: number;
  encoded_application?: string;
  encoded_library?: string;
  title?: string;
  artist?: string;
  album?: string;
  recorded_date?: string;
  genre?: string;
  track_position?: string;
  cover_art_present: boolean;
  cover_mime?: string;
  cover_data_base64?: string;
  header_size?: number;
  data_size?: number;
  footer_size?: number;
  is_streamable?: boolean;
}

export interface VideoTrack {
  stream_id: number;
  stream_order?: number;
  format: string;
  format_info?: string;
  format_profile?: string;
  format_level?: string;
  codec_id?: string;
  duration_ms?: number;
  bit_rate?: number;
  bit_rate_mode?: string;
  width: number;
  height: number;
  display_aspect_ratio?: string;
  frame_rate?: number;
  frame_rate_mode?: string;
  frame_count?: number;
  color_space?: string;
  color_encoding?: string;
  chroma_subsampling?: string;
  bit_depth?: number;
  scan_type?: string;
  color_range?: string;
  color_primaries?: string;
  transfer_characteristics?: string;
  matrix_coefficients?: string;
  mastering_display_color_primaries?: string;
  mastering_display_luminance?: string;
  maximum_content_light_level?: number;
  maximum_frame_average_light_level?: number;
  dolby_vision_version?: string;
  dolby_vision_profile?: number;
  dolby_vision_level?: number;
  dolby_vision_rpu_present?: boolean;
  hdr_format?: string;
  title?: string;
  language?: string;
  default_flag: boolean;
  forced_flag: boolean;
}

export interface AudioTrack {
  stream_id: number;
  stream_order?: number;
  format: string;
  format_info?: string;
  format_profile?: string;
  codec_id?: string;
  duration_ms?: number;
  bit_rate?: number;
  bit_rate_mode?: string;
  channels: number;
  channel_layout?: string;
  sampling_rate: number;
  sampling_count?: number;
  bit_depth?: number;
  compression_mode?: string;
  delay_relative_to_video_ms?: number;
  title?: string;
  language?: string;
  default_flag: boolean;
  forced_flag: boolean;
  dolby_atmos_present?: boolean;
  dts_x_present?: boolean;
}

export interface TextTrack {
  stream_id: number;
  stream_order?: number;
  format: string;
  format_info?: string;
  codec_id?: string;
  duration_ms?: number;
  element_count?: number;
  title?: string;
  language?: string;
  default_flag: boolean;
  forced_flag: boolean;
}

export interface Chapter {
  timestamp_ms: number;
  title: string;
}

export interface MenuTrack {
  chapters: Chapter[];
}

export interface Attachment {
  id: number;
  file_name: string;
  mime_type: string;
  data_size: number;
  description?: string;
}

export interface BitstreamNode {
  name: string;
  offset: number;
  size: number;
  description?: string;
  children: BitstreamNode[];
}

export interface MediaReport {
  general: GeneralTrack;
  videos: VideoTrack[];
  audios: AudioTrack[];
  texts: TextTrack[];
  menu?: MenuTrack;
  attachments: Attachment[];
  bitstream_root?: BitstreamNode;
}

export interface FieldDiff {
  category: string;
  field: string;
  value_a: string;
  value_b: string;
}

export interface ComparisonDiff {
  file_a: string;
  file_b: string;
  report_a: MediaReport;
  report_b: MediaReport;
  differences: FieldDiff[];
}
