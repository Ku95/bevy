use bevy_macro_utils::BevyManifest;
use encase_derive_impl::{implement, syn};

const BEVY: &str = "bevy";
const BEVY_GPU: &str = "bevy_gpu";
const ENCASE: &str = "encase";

fn bevy_encase_path() -> syn::Path {
    let bevy_manifest = BevyManifest::default();
    bevy_manifest
        .maybe_get_path(BEVY)
        .map(|bevy_path| {
            let mut segments = bevy_path.segments;
            segments.push(BevyManifest::parse_str("render"));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .or_else(|| bevy_manifest.maybe_get_path(BEVY_GPU))
        .map(|bevy_gpu_path| {
            let mut segments = bevy_gpu_path.segments;
            segments.push(BevyManifest::parse_str("gpu_resource"));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .map(|path| {
            let mut segments = path.segments;
            segments.push(BevyManifest::parse_str(ENCASE));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .unwrap_or_else(|| bevy_manifest.get_path(ENCASE))
}

implement!(bevy_encase_path());
