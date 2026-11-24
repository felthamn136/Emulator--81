use super::*;
use crate::wl_request;

#[derive(Default)]
pub struct WaylandDnD {
    data_offer: Option<*mut wl_data_offer>,
    enter_serial: Option<core::ffi::c_uint>,
    pub(super) offers: std::collections::HashMap<*mut wl_data_offer, &'static str>,
    mime: Option<&'static str>,
}

pub(super) unsafe extern "C" fn data_offer_handle_offer(
    data: *mut ::core::ffi::c_void,
    data_offer: *mut wl_data_offer,
    mime_type: *const ::core::ffi::c_char,
) {
    let display: &mut WaylandPayload = &mut *(data as *mut _);
    let mime = std::ffi::CStr::from_ptr(mime_type).to_bytes();
    if mime == b"text/uri-list" {
        display.drag_n_drop.offers.insert(data_offer, "text/uri-list");
    } else if mime == b"UTF8_STRING" {
        display.drag_n_drop.offers.entry(data_offer).or_insert("UTF8_STRING");
    }
}

pub(super) unsafe extern "C" fn data_offer_handle_source_actions(
    data: *mut ::core::ffi::c_void,
    data_offer: *mut wl_data_offer,
    actions: ::core::ffi::c_uint,
) {
    if actions & WL_DATA_DEVICE_MANAGER_DND_ACTION_COPY == 1 {
        let display: &mut WaylandPayload = &mut *(data as *mut _);
        wl_request!(
            display.client,
            data_offer,
            WL_DATA_OFFER_SET_ACTIONS,
            WL_DATA_DEVICE_MANAGER_DND_ACTION_COPY,
            WL_DATA_DEVICE_MANAGER_DND_ACTION_COPY
        );
    }
}

pub(super) unsafe extern "C" fn data_device_handle_enter(
    data: *mut ::core::ffi::c_void,
    data_device: *mut wl_data_device,
    serial: core::ffi::c_uint,
    _surface: *mut wl_surface,
    _surface_x: i32,
    _surface_y: i32,
    data_offer: *mut wl_data_offer,
) {
    let display: &mut WaylandPayload = &mut *(data as *mut _);
    assert_eq!(data_device, display.data_device);
    display.drag_n_drop.enter_serial = Some(serial);
    display.drag_n_drop.data_offer = (!data_offer.is_null()).then_some(data_offer);
    display.drag_n_drop.mime = display.drag_n_drop.offers.remove(&data_offer);
    if let Some(mime) = display.drag_n_drop.mime {
        let mime_type = std::ffi::CString::new(mime).unwrap();
        wl_request!(
            display.client,
            data_offer,
            WL_DATA_OFFER_ACCEPT,
            serial,
            mime_type.as_ptr()
        );
    }
}

pub(super) unsafe extern "C" fn data_device_handle_leave(
    data: *mut ::core::ffi::c_void,
    data_device: *mut wl_data_device,
) {
    let display: &mut WaylandPayload = &mut *(data as *mut _);
    assert_eq!(data_device, display.data_device);
    display.drag_n_drop.enter_serial = None;
    if let Some(data_offer) = display.drag_n_drop.data_offer.take() {
        wl_request!(display.client, data_offer, WL_DATA_OFFER_DESTROY);
        (display.client.wl_proxy_destroy)(data_offer as _);
    }
    display.drag_n_drop.mime = None;
}

pub(super) unsafe extern "C" fn data_device_handle_drop(
    data: *mut ::core::ffi::c_void,
    data_device: *mut wl_data_device,
) {
    let display: &mut WaylandPayload = &mut *(data as *mut _);
    assert_eq!(data_device, display.data_device);
    if let Some(data_offer) = display.drag_n_drop.data_offer {
        if let Some(mime) = display.drag_n_drop.mime.take() {
            let mime_type = std::ffi::CString::new(mime).unwrap();
            if let Some(bytes) =
                display
                    .client
                    .data_offer_receive(display.display, data_offer, mime_type.as_ptr())
            {
                match String::from_utf8(bytes) {
                    Ok(filenames) => display.events.push(WaylandEvent::FilesDropped(filenames)),
                    Err(error) => eprintln!("Cannot decode dropped file list: {error}"),
                }
            } else {
                eprintln!("Cannot receive dropped file list");
            }
        }
        wl_request!(display.client, data_offer, WL_DATA_OFFER_DESTROY);
        (display.client.wl_proxy_destroy)(data_offer as _);
        display.drag_n_drop.data_offer = None;
    }
}
