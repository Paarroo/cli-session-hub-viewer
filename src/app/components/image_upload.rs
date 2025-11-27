//! Image upload components for chat
//!
//! Includes drag & drop zone, image gallery, and lightbox components.

use dioxus::prelude::*;
use crate::domain::models::{ImageAttachment, MAX_IMAGES_PER_MESSAGE};
use crate::shared::hooks::ImageUploadState;

/// Props for ImageUploadZone
#[derive(Props, Clone, PartialEq)]
pub struct ImageUploadZoneProps {
    /// Image upload state from hook
    pub upload_state: ImageUploadState,
    /// Whether the zone is disabled
    #[props(default = false)]
    pub disabled: bool,
    /// Compact mode (smaller zone)
    #[props(default = false)]
    pub compact: bool,
}

/// Drag & drop zone for image uploads
#[component]
pub fn ImageUploadZone(props: ImageUploadZoneProps) -> Element {
    let mut is_drag_over = use_signal(|| false);
    let upload_state = props.upload_state.clone();

    let can_add = upload_state.can_add_more();
    let total_count = upload_state.total_count();

    rsx! {
        div {
            class: if props.compact { "image-upload-zone image-upload-zone--compact" } else { "image-upload-zone" },
            class: if *is_drag_over.read() { "image-upload-zone--drag-over" } else { "" },
            class: if props.disabled || !can_add { "image-upload-zone--disabled" } else { "" },

            // Drag & drop events
            ondragover: move |evt| {
                evt.prevent_default();
                is_drag_over.set(true);
            },
            ondragleave: move |_| {
                is_drag_over.set(false);
            },
            ondrop: move |evt| {
                evt.prevent_default();
                is_drag_over.set(false);
                // File handling done via file input for better compatibility
            },

            // Hidden file input (uses JavaScript bridge for file handling)
            input {
                r#type: "file",
                accept: "image/jpeg,image/png,image/webp,image/gif",
                multiple: true,
                id: "image-upload-input",
                class: "image-upload-zone__input",
                disabled: props.disabled || !can_add,
                // Note: File handling is done via __setupImageFileInput JavaScript bridge
                // which calls __onFilesDropped callback set up in chat_input.rs
            }

            // Upload label/button
            label {
                r#for: "image-upload-input",
                class: "image-upload-zone__label",

                // Icon
                div {
                    class: "image-upload-zone__icon",
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "24",
                        height: "24",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
                        polyline { points: "17 8 12 3 7 8" }
                        line { x1: "12", y1: "3", x2: "12", y2: "15" }
                    }
                }

                // Text
                if props.compact {
                    span {
                        class: "image-upload-zone__text",
                        "Ajouter des images"
                    }
                } else {
                    div {
                        class: "image-upload-zone__text",
                        "Glisser-déposer des images ici"
                    }
                    div {
                        class: "image-upload-zone__subtext",
                        "ou cliquer pour sélectionner"
                    }
                }

                // Counter
                if total_count > 0 {
                    div {
                        class: "image-upload-zone__counter",
                        "{total_count}/{MAX_IMAGES_PER_MESSAGE}"
                    }
                }
            }
        }
    }
}


/// Props for ImagePreviewGrid
#[derive(Props, Clone, PartialEq)]
pub struct ImagePreviewGridProps {
    /// Image upload state
    pub upload_state: ImageUploadState,
    /// Allow removal of images
    #[props(default = true)]
    pub removable: bool,
}

/// Grid of image previews (pending + uploaded)
#[component]
pub fn ImagePreviewGrid(props: ImagePreviewGridProps) -> Element {
    let upload_state = props.upload_state.clone();
    let pending = upload_state.pending_images.read();
    let uploaded = upload_state.uploaded_images.read();

    let has_images = !pending.is_empty() || !uploaded.is_empty();

    if !has_images {
        return rsx! {};
    }

    rsx! {
        div {
            class: "image-preview-grid",

            // Pending images
            for img in pending.iter() {
                ImagePreviewItem {
                    key: "{img.id}",
                    id: img.id.clone(),
                    url: img.preview_url.clone(),
                    filename: img.filename.clone(),
                    is_pending: true,
                    progress: img.upload_progress,
                    error: img.error.clone(),
                    removable: props.removable,
                    on_remove: {
                        let id = img.id.clone();
                        let mut state = upload_state.clone();
                        move |_| {
                            state.remove_pending(&id);
                        }
                    },
                }
            }

            // Uploaded images
            for img in uploaded.iter() {
                ImagePreviewItem {
                    key: "{img.id}",
                    id: img.id.clone(),
                    url: img.url.clone(),
                    filename: img.filename.clone(),
                    is_pending: false,
                    progress: 1.0,
                    error: None,
                    removable: props.removable,
                    on_remove: {
                        let id = img.id.clone();
                        let mut state = upload_state.clone();
                        move |_| {
                            state.remove_uploaded(&id);
                        }
                    },
                }
            }
        }
    }
}

/// Props for ImagePreviewItem
#[derive(Props, Clone, PartialEq)]
pub struct ImagePreviewItemProps {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub is_pending: bool,
    pub progress: f32,
    pub error: Option<String>,
    pub removable: bool,
    pub on_remove: EventHandler<()>,
}

/// Single image preview item
#[component]
pub fn ImagePreviewItem(props: ImagePreviewItemProps) -> Element {
    rsx! {
        div {
            class: "image-preview-item",
            class: if props.is_pending { "image-preview-item--pending" } else { "" },
            class: if props.error.is_some() { "image-preview-item--error" } else { "" },

            // Image
            img {
                src: "{props.url}",
                alt: "{props.filename}",
                class: "image-preview-item__image",
            }

            // Progress overlay
            if props.is_pending && props.progress < 1.0 {
                div {
                    class: "image-preview-item__progress",
                    div {
                        class: "image-preview-item__progress-bar",
                        style: "width: {props.progress * 100.0}%",
                    }
                }
            }

            // Error overlay
            if let Some(error) = &props.error {
                div {
                    class: "image-preview-item__error",
                    title: "{error}",
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        circle { cx: "12", cy: "12", r: "10" }
                        line { x1: "12", y1: "8", x2: "12", y2: "12" }
                        line { x1: "12", y1: "16", x2: "12.01", y2: "16" }
                    }
                }
            }

            // Remove button
            if props.removable {
                button {
                    class: "image-preview-item__remove",
                    title: "Supprimer",
                    onclick: move |_| props.on_remove.call(()),
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "14",
                        height: "14",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        line { x1: "18", y1: "6", x2: "6", y2: "18" }
                        line { x1: "6", y1: "6", x2: "18", y2: "18" }
                    }
                }
            }
        }
    }
}

/// Props for ImageGallery (display in messages)
#[derive(Props, Clone, PartialEq)]
pub struct ImageGalleryProps {
    /// Images to display
    pub images: Vec<ImageAttachment>,
    /// Enable lightbox on click
    #[props(default = true)]
    pub lightbox_enabled: bool,
}

/// Image gallery for displaying images in messages
#[component]
pub fn ImageGallery(props: ImageGalleryProps) -> Element {
    let mut selected_index = use_signal(|| None::<usize>);

    if props.images.is_empty() {
        return rsx! {};
    }

    let image_count = props.images.len();
    let grid_class = match image_count {
        1 => "image-gallery image-gallery--single",
        2 => "image-gallery image-gallery--double",
        _ => "image-gallery image-gallery--grid",
    };

    rsx! {
        div {
            class: "{grid_class}",

            for (index, img) in props.images.iter().enumerate() {
                div {
                    key: "{img.id}",
                    class: "image-gallery__item",
                    onclick: move |_| {
                        if props.lightbox_enabled {
                            selected_index.set(Some(index));
                        }
                    },

                    img {
                        src: "{img.url}",
                        alt: "{img.filename}",
                        class: "image-gallery__image",
                        loading: "lazy",
                    }
                }
            }
        }

        // Lightbox
        if let Some(index) = *selected_index.read() {
            ImageLightbox {
                images: props.images.clone(),
                initial_index: index,
                on_close: move |_| selected_index.set(None),
            }
        }
    }
}

/// Props for ImageLightbox
#[derive(Props, Clone, PartialEq)]
pub struct ImageLightboxProps {
    pub images: Vec<ImageAttachment>,
    pub initial_index: usize,
    pub on_close: EventHandler<()>,
}

/// Fullscreen image lightbox
#[component]
pub fn ImageLightbox(props: ImageLightboxProps) -> Element {
    let mut current_index = use_signal(|| props.initial_index);
    let image_count = props.images.len();

    let can_prev = *current_index.read() > 0;
    let can_next = *current_index.read() < image_count - 1;

    let current_image = props.images.get(*current_index.read());

    rsx! {
        div {
            class: "image-lightbox",
            onclick: move |_| props.on_close.call(()),

            // Navigation - Previous
            if can_prev {
                button {
                    class: "image-lightbox__nav image-lightbox__nav--prev",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        current_index -= 1;
                    },
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "32",
                        height: "32",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        polyline { points: "15 18 9 12 15 6" }
                    }
                }
            }

            // Main image
            div {
                class: "image-lightbox__content",
                onclick: move |evt| evt.stop_propagation(),

                if let Some(img) = current_image {
                    img {
                        src: "{img.url}",
                        alt: "{img.filename}",
                        class: "image-lightbox__image",
                    }

                    // Image info
                    {
                        let current_idx = *current_index.read() + 1;
                        rsx! {
                            div {
                                class: "image-lightbox__info",
                                span { "{img.filename}" }
                                if image_count > 1 {
                                    span {
                                        class: "image-lightbox__counter",
                                        "{current_idx} / {image_count}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Navigation - Next
            if can_next {
                button {
                    class: "image-lightbox__nav image-lightbox__nav--next",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        current_index += 1;
                    },
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "32",
                        height: "32",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        polyline { points: "9 18 15 12 9 6" }
                    }
                }
            }

            // Close button
            button {
                class: "image-lightbox__close",
                onclick: move |_| props.on_close.call(()),
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "24",
                    height: "24",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    line { x1: "18", y1: "6", x2: "6", y2: "18" }
                    line { x1: "6", y1: "6", x2: "18", y2: "18" }
                }
            }
        }
    }
}

/// Upload button (compact, for chat input)
#[derive(Props, Clone, PartialEq)]
pub struct ImageUploadButtonProps {
    pub upload_state: ImageUploadState,
    #[props(default = false)]
    pub disabled: bool,
}

#[component]
pub fn ImageUploadButton(props: ImageUploadButtonProps) -> Element {
    let upload_state = props.upload_state.clone();
    let can_add = upload_state.can_add_more();
    let count = upload_state.total_count();

    rsx! {
        label {
            class: "image-upload-button",
            class: if props.disabled || !can_add { "image-upload-button--disabled" } else { "" },
            title: if can_add { "Ajouter des images" } else { "Maximum d'images atteint" },

            input {
                r#type: "file",
                accept: "image/jpeg,image/png,image/webp,image/gif",
                multiple: true,
                class: "image-upload-button__input",
                disabled: props.disabled || !can_add,
            }

            svg {
                xmlns: "http://www.w3.org/2000/svg",
                width: "20",
                height: "20",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                rect { x: "3", y: "3", width: "18", height: "18", rx: "2", ry: "2" }
                circle { cx: "8.5", cy: "8.5", r: "1.5" }
                polyline { points: "21 15 16 10 5 21" }
            }

            if count > 0 {
                span {
                    class: "image-upload-button__badge",
                    "{count}"
                }
            }
        }
    }
}
