use std::borrow::Cow;
use std::cell::RefCell;
use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;

use relm4::adw;
use relm4::gtk::{self, cairo, glib, prelude::*};

use crate::config::key_binding::KeyBinding;
use crate::config::lighting::{LedConfig, LightMode, LightSpeed, RgbColor};
use crate::device::{CanvasTransform, DeviceCapabilities, DeviceLayout, DeviceModel, ElementShape};

const FRAME_INTERVAL_US: i64 = 16_000; // ~60 FPS cap for smooth, low-overhead previews
const MANUAL_DEMO_COOLDOWN_US: i64 = 2_000_000;
const KEY_SLOTS_CAP: usize = 8;

/// Normalized RGBA color helper for Cairo rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl Rgba {
    const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    fn from_rgb(rgb: RgbColor, a: f64) -> Self {
        Self {
            r: rgb.r as f64 / 255.0,
            g: rgb.g as f64 / 255.0,
            b: rgb.b as f64 / 255.0,
            a: a.clamp(0.0, 1.0),
        }
    }

    fn apply(self, cr: &cairo::Context) {
        cr.set_source_rgba(self.r, self.g, self.b, self.a);
    }
}

#[derive(Debug, Clone, Copy)]
struct PreviewPalette {
    shadow: Rgba,
    chassis_fill: Rgba,
    chassis_border: Rgba,
    usb_muted: Rgba,
    key_fill: Rgba,
    key_hover_fill: Rgba,
    key_border: Rgba,
    key_selected_fill: Rgba,
    key_selected_border: Rgba,
    label_primary: Rgba,
    label_muted: Rgba,
    label_accent: Rgba,
    lighting_chassis_fill: Rgba,
    lighting_key_tint: Rgba,
    led_off_fill: Rgba,
}

impl PreviewPalette {
    fn for_theme(dark: bool) -> Self {
        if dark {
            Self {
                shadow: Rgba::new(0.0, 0.0, 0.0, 0.30),
                chassis_fill: Rgba::new(0.16, 0.17, 0.20, 1.0),
                chassis_border: Rgba::new(0.32, 0.35, 0.40, 1.0),
                usb_muted: Rgba::new(0.68, 0.72, 0.78, 0.85),
                key_fill: Rgba::new(0.22, 0.24, 0.28, 1.0),
                key_hover_fill: Rgba::new(0.27, 0.30, 0.35, 1.0),
                key_border: Rgba::new(0.38, 0.42, 0.48, 1.0),
                key_selected_fill: Rgba::new(0.18, 0.35, 0.56, 0.65),
                key_selected_border: Rgba::new(0.38, 0.76, 1.0, 1.0),
                label_primary: Rgba::new(0.95, 0.96, 0.98, 1.0),
                label_muted: Rgba::new(0.70, 0.73, 0.79, 0.95),
                label_accent: Rgba::new(0.55, 0.84, 1.0, 1.0),
                lighting_chassis_fill: Rgba::new(0.11, 0.12, 0.14, 1.0),
                lighting_key_tint: Rgba::new(0.18, 0.20, 0.23, 0.42),
                led_off_fill: Rgba::new(0.15, 0.16, 0.19, 0.85),
            }
        } else {
            Self {
                shadow: Rgba::new(0.0, 0.0, 0.0, 0.10),
                chassis_fill: Rgba::new(0.93, 0.94, 0.96, 1.0),
                chassis_border: Rgba::new(0.72, 0.75, 0.80, 1.0),
                usb_muted: Rgba::new(0.36, 0.40, 0.46, 0.90),
                key_fill: Rgba::new(1.0, 1.0, 1.0, 1.0),
                key_hover_fill: Rgba::new(0.95, 0.97, 1.0, 1.0),
                key_border: Rgba::new(0.68, 0.72, 0.78, 1.0),
                key_selected_fill: Rgba::new(0.08, 0.45, 0.85, 0.14),
                key_selected_border: Rgba::new(0.0, 0.42, 0.78, 1.0),
                label_primary: Rgba::new(0.11, 0.13, 0.16, 1.0),
                label_muted: Rgba::new(0.42, 0.46, 0.52, 0.95),
                label_accent: Rgba::new(0.0, 0.40, 0.75, 1.0),
                lighting_chassis_fill: Rgba::new(0.16, 0.18, 0.22, 1.0),
                lighting_key_tint: Rgba::new(0.24, 0.26, 0.31, 0.45),
                led_off_fill: Rgba::new(0.20, 0.22, 0.26, 0.85),
            }
        }
    }
}

fn is_dark_theme() -> bool {
    adw::StyleManager::default().is_dark()
}

fn connect_theme_redraw(area: &gtk::DrawingArea) {
    let weak = area.downgrade();
    let sig_id = adw::StyleManager::default().connect_dark_notify(move |_| {
        if let Some(area) = weak.upgrade() {
            area.queue_draw();
        }
    });
    let sig_cell = Rc::new(RefCell::new(Some(sig_id)));
    area.connect_destroy(move |_| {
        if let Some(id) = sig_cell.borrow_mut().take() {
            adw::StyleManager::default().disconnect(id);
        }
    });
}

fn hit_test_widget_key(
    widget: &gtk::Widget,
    model: DeviceModel,
    sx: f64,
    sy: f64,
) -> Option<usize> {
    let layout = model.layout();
    let tx = CanvasTransform::fit(widget.width() as f64, widget.height() as f64, layout, 14.0);
    let (bx, by) = tx.to_blueprint(sx, sy);
    layout.hit_test_key(bx, by)
}

fn format_key_tooltip(idx: usize, keys: &[KeyBinding]) -> String {
    let binding = keys.get(idx).copied().unwrap_or_default();
    format!("BT{} · {}  (Click to edit)", idx + 1, binding)
}

fn ui_font_family() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Segoe UI"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "Sans"
    }
}

fn rounded_rect_path(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
    let x2 = x + w;
    let y2 = y + h;
    cr.new_sub_path();
    cr.arc(x2 - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x2 - r, y2 - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y2 - r, r, FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * FRAC_PI_2);
    cr.close_path();
}

#[derive(Debug, Clone, Copy)]
struct ElementGeom {
    cx: f64,
    cy: f64,
    width: f64,
    height: f64,
    shape: ElementShape,
    radius: f64,
}

impl ElementGeom {
    const fn from_key(slot: &crate::device::KeySlotLayout) -> Self {
        Self {
            cx: slot.cx,
            cy: slot.cy,
            width: slot.width,
            height: slot.height,
            shape: slot.shape,
            radius: 6.0,
        }
    }

    const fn from_led(zone: &crate::device::LedZoneLayout) -> Self {
        Self {
            cx: zone.cx,
            cy: zone.cy,
            width: zone.width,
            height: zone.height,
            shape: zone.shape,
            radius: 6.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TextStyle {
    size: f64,
    bold: bool,
    color: Rgba,
}

fn build_element_path(cr: &cairo::Context, tx: CanvasTransform, geom: ElementGeom) {
    let sx = tx.to_screen_x(geom.cx);
    let sy = tx.to_screen_y(geom.cy);
    let sw = tx.len(geom.width).max(1.0);
    let sh = tx.len(geom.height).max(1.0);

    match geom.shape {
        ElementShape::RoundedRect => {
            rounded_rect_path(
                cr,
                sx - sw * 0.5,
                sy - sh * 0.5,
                sw,
                sh,
                tx.len(geom.radius).clamp(2.5, 10.0),
            );
        }
        ElementShape::Circle => {
            let _ = cr.save();
            cr.translate(sx, sy);
            cr.scale(sw * 0.5, sh * 0.5);
            cr.new_path();
            cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
            cr.close_path();
            let _ = cr.restore();
        }
        ElementShape::RockerRight => {
            let left_x = sx - sw * 0.5;
            let _ = cr.save();
            cr.translate(left_x, sy);
            cr.scale(sw, sh * 0.5);
            cr.new_path();
            cr.move_to(0.0, -1.0);
            cr.arc(0.0, 0.0, 1.0, -FRAC_PI_2, FRAC_PI_2);
            cr.close_path();
            let _ = cr.restore();
        }
    }
}

fn draw_centered_text(
    cr: &cairo::Context,
    cx: f64,
    cy: f64,
    max_w: f64,
    text: &str,
    style: TextStyle,
) {
    if text.is_empty() || max_w <= 4.0 {
        return;
    }

    let weight = if style.bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };
    cr.select_font_face(ui_font_family(), cairo::FontSlant::Normal, weight);
    style.color.apply(cr);

    let mut chosen_size = style.size.max(7.5);
    cr.set_font_size(chosen_size);
    if let Ok(ext) = cr.text_extents(text)
        && ext.width() > max_w
    {
        chosen_size = (style.size * 0.80).max(7.5);
        cr.set_font_size(chosen_size);
    }

    let fitted = fit_text_with_ellipsis(cr, text, max_w);
    if let Ok(ext) = cr.text_extents(&fitted) {
        let tx = cx - (ext.x_bearing() + ext.width() * 0.5);
        let ty = cy - (ext.y_bearing() + ext.height() * 0.5);
        cr.move_to(tx, ty);
        let _ = cr.show_text(&fitted);
    }
}

fn fit_text_with_ellipsis<'a>(cr: &cairo::Context, text: &'a str, max_w: f64) -> Cow<'a, str> {
    if let Ok(ext) = cr.text_extents(text)
        && ext.width() <= max_w
    {
        return Cow::Borrowed(text);
    }

    let mut best_end = 0usize;
    let mut buf = String::with_capacity(text.len() + 3);
    for (idx, _) in text.char_indices().skip(1) {
        buf.clear();
        buf.push_str(&text[..idx]);
        buf.push('…');
        if let Ok(ext) = cr.text_extents(&buf)
            && ext.width() <= max_w
        {
            best_end = idx;
        } else {
            break;
        }
    }

    if best_end == 0 {
        Cow::Borrowed("…")
    } else {
        Cow::Owned(format!("{}…", &text[..best_end]))
    }
}

fn draw_chassis_and_usb(
    cr: &cairo::Context,
    tx: CanvasTransform,
    layout: &DeviceLayout,
    fill: Rgba,
    border: Rgba,
    palette: PreviewPalette,
) {
    let bx = tx.to_screen_x(0.0);
    let by = tx.to_screen_y(0.0);
    let bw = tx.len(layout.body_width);
    let bh = tx.len(layout.body_height);
    let radius = tx.len(layout.corner_radius).max(6.0);

    // Subtle elevation shadow under keypad chassis
    rounded_rect_path(cr, bx, by + 2.5, bw, bh, radius);
    palette.shadow.apply(cr);
    let _ = cr.fill();

    // Main keypad body
    rounded_rect_path(cr, bx, by, bw, bh, radius);
    fill.apply(cr);
    let _ = cr.fill_preserve();
    border.apply(cr);
    cr.set_line_width(1.5);
    let _ = cr.stroke();

    if layout.has_usb_indicator {
        let mid_x = tx.to_screen_x(layout.body_width * 0.5);
        let base_y = tx.to_screen_y(-4.0);
        let tri_h = tx.len(7.5).clamp(5.0, 10.0);
        let tri_half_w = tx.len(7.0).clamp(5.0, 10.0);

        // "USB" label just above chassis top edge, arrow tip above that
        let label_y = base_y - tx.len(5.5);
        draw_centered_text(
            cr,
            mid_x,
            label_y,
            tx.len(40.0),
            "USB",
            TextStyle {
                size: (tx.scale * 7.8).clamp(8.0, 11.0),
                bold: false,
                color: palette.usb_muted,
            },
        );

        let apex_base = label_y - tx.len(6.0);
        cr.new_path();
        cr.move_to(mid_x, apex_base - tri_h);
        cr.line_to(mid_x - tri_half_w, apex_base);
        cr.line_to(mid_x + tri_half_w, apex_base);
        cr.close_path();
        palette.usb_muted.apply(cr);
        cr.set_line_width(1.25);
        let _ = cr.stroke();
    }
}

fn draw_rocker_arrows(
    cr: &cairo::Context,
    tx: CanvasTransform,
    cx: f64,
    cy: f64,
    height: f64,
    color: Rgba,
) {
    let sx = tx.to_screen_x(cx);
    let top_y = tx.to_screen_y(cy - height * 0.5 - 6.0);
    let bot_y = tx.to_screen_y(cy + height * 0.5 + 6.0);
    let arrow_r = tx.len(4.5).clamp(3.0, 6.0);
    let font_size = (tx.scale * 7.0).clamp(7.5, 9.5);

    color.apply(cr);
    cr.set_line_width(1.2);

    // Up triangle + UP caption
    cr.new_path();
    cr.move_to(sx, top_y - arrow_r);
    cr.line_to(sx - arrow_r, top_y + arrow_r * 0.5);
    cr.line_to(sx + arrow_r, top_y + arrow_r * 0.5);
    cr.close_path();
    let _ = cr.stroke();
    let label_style = TextStyle {
        size: font_size,
        bold: false,
        color,
    };
    draw_centered_text(
        cr,
        sx,
        top_y - arrow_r - tx.len(6.5),
        tx.len(26.0),
        "UP",
        label_style,
    );

    // Down triangle + DN caption
    cr.new_path();
    cr.move_to(sx, bot_y + arrow_r);
    cr.line_to(sx - arrow_r, bot_y - arrow_r * 0.5);
    cr.line_to(sx + arrow_r, bot_y - arrow_r * 0.5);
    cr.close_path();
    let _ = cr.stroke();
    draw_centered_text(
        cr,
        sx,
        bot_y + arrow_r + tx.len(6.5),
        tx.len(26.0),
        "DN",
        label_style,
    );
}

// -----------------------------------------------------------------------------
// Keys View Interactive Hardware Diagram
// -----------------------------------------------------------------------------

struct KeysDiagramState {
    model: DeviceModel,
    keys: Vec<KeyBinding>,
    selected_idx: usize,
    hovered_idx: Option<usize>,
}

/// Interactive Cairo hardware blueprint showing key positions and live bindings.
pub struct KeysDiagramPreview {
    area: gtk::DrawingArea,
    state: Rc<RefCell<KeysDiagramState>>,
}

impl KeysDiagramPreview {
    pub fn new(on_select: impl Fn(usize) + 'static) -> Self {
        let area = gtk::DrawingArea::builder()
            .hexpand(true)
            .vexpand(false)
            .content_width(280)
            .content_height(190)
            .build();

        let state = Rc::new(RefCell::new(KeysDiagramState {
            model: DeviceModel::SimpadNanoYearEdition,
            keys: Vec::new(),
            selected_idx: 0,
            hovered_idx: None,
        }));

        // Render hardware blueprint & key slots
        let state_draw = Rc::clone(&state);
        area.set_draw_func(move |_area, cr, width, height| {
            let st = state_draw.borrow();
            draw_keys_diagram(cr, width as f64, height as f64, &st);
        });

        // Click anywhere inside a physical key shape to select it
        let click = gtk::GestureClick::builder().button(1).build();
        let state_click = Rc::clone(&state);
        click.connect_pressed(move |gesture, _, x, y| {
            let Some(widget) = gesture.widget() else {
                return;
            };
            let hit = {
                let st = state_click.borrow();
                hit_test_widget_key(&widget, st.model, x, y)
            };
            if let Some(idx) = hit {
                on_select(idx);
            }
        });
        area.add_controller(click);

        // Hover feedback & full-shortcut tooltip on compact keys
        let motion = gtk::EventControllerMotion::new();
        let state_move = Rc::clone(&state);
        motion.connect_motion(move |ctrl, x, y| {
            let Some(widget) = ctrl.widget() else {
                return;
            };
            let (changed, new_hover, tooltip) = {
                let mut st = state_move.borrow_mut();
                let hit = hit_test_widget_key(&widget, st.model, x, y);
                let changed = st.hovered_idx != hit;
                if changed {
                    st.hovered_idx = hit;
                }
                let tip = hit.map(|idx| format_key_tooltip(idx, &st.keys));
                (changed, hit, tip)
            };
            if changed {
                widget.set_cursor_from_name(new_hover.map(|_| "pointer"));
                widget.set_tooltip_text(tooltip.as_deref());
                widget.queue_draw();
            }
        });

        let state_leave = Rc::clone(&state);
        motion.connect_leave(move |ctrl| {
            let Some(widget) = ctrl.widget() else {
                return;
            };
            let changed = {
                let mut st = state_leave.borrow_mut();
                let had = st.hovered_idx.is_some();
                st.hovered_idx = None;
                had
            };
            if changed {
                widget.set_cursor_from_name(None);
                widget.set_tooltip_text(None);
                widget.queue_draw();
            }
        });
        area.add_controller(motion);

        connect_theme_redraw(&area);

        Self { area, state }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.area
    }

    pub fn sync(&self, model: DeviceModel, keys: &[KeyBinding], selected_idx: usize) {
        let (changed, refreshed_tooltip) = {
            let mut st = self.state.borrow_mut();
            let diff =
                st.model != model || st.selected_idx != selected_idx || st.keys.as_slice() != keys;
            if diff {
                st.model = model;
                st.keys.clear();
                st.keys.extend_from_slice(keys);
                st.selected_idx = selected_idx;
                let tip = st
                    .hovered_idx
                    .filter(|&i| i < st.keys.len())
                    .map(|idx| format_key_tooltip(idx, &st.keys));
                (true, Some(tip))
            } else {
                (false, None)
            }
        };
        if let Some(tip) = refreshed_tooltip {
            self.area.set_tooltip_text(tip.as_deref());
        }
        if changed {
            self.area.queue_draw();
        }
    }
}

fn draw_keys_diagram(cr: &cairo::Context, canvas_w: f64, canvas_h: f64, st: &KeysDiagramState) {
    draw_keys_diagram_with_palette(
        cr,
        canvas_w,
        canvas_h,
        st,
        PreviewPalette::for_theme(is_dark_theme()),
    );
}

fn draw_keys_diagram_with_palette(
    cr: &cairo::Context,
    canvas_w: f64,
    canvas_h: f64,
    st: &KeysDiagramState,
    palette: PreviewPalette,
) {
    let layout = st.model.layout();
    if layout.keys.is_empty() {
        return;
    }

    let tx = CanvasTransform::fit(canvas_w, canvas_h, layout, 14.0);

    draw_chassis_and_usb(
        cr,
        tx,
        layout,
        palette.chassis_fill,
        palette.chassis_border,
        palette,
    );

    for slot in layout.keys {
        let is_selected = slot.index == st.selected_idx;
        let is_hovered = st.hovered_idx == Some(slot.index);
        let binding = st.keys.get(slot.index).copied().unwrap_or_default();

        if slot.show_rocker_arrows {
            draw_rocker_arrows(
                cr,
                tx,
                slot.cx,
                slot.cy,
                slot.height,
                if is_selected {
                    palette.label_accent
                } else {
                    palette.label_muted
                },
            );
        }

        let geom = ElementGeom::from_key(slot);

        // Subtle key shadow on larger keys
        if slot.height >= 40.0 {
            let _ = cr.save();
            cr.translate(0.0, 1.5);
            build_element_path(cr, tx, geom);
            palette.shadow.apply(cr);
            let _ = cr.fill();
            let _ = cr.restore();
        }

        build_element_path(cr, tx, geom);

        let fill = if is_selected {
            palette.key_selected_fill
        } else if is_hovered {
            palette.key_hover_fill
        } else {
            palette.key_fill
        };
        fill.apply(cr);
        let _ = cr.fill_preserve();

        let (border, line_w) = if is_selected {
            (palette.key_selected_border, 2.2)
        } else if is_hovered {
            (palette.key_selected_border, 1.4)
        } else {
            (palette.key_border, 1.25)
        };
        border.apply(cr);
        cr.set_line_width(line_w);
        let _ = cr.stroke();

        draw_keycap_labels(cr, tx, slot, binding, is_selected, palette);
    }
}

fn draw_keycap_labels(
    cr: &cairo::Context,
    tx: CanvasTransform,
    slot: &crate::device::KeySlotLayout,
    binding: KeyBinding,
    is_selected: bool,
    palette: PreviewPalette,
) {
    let sx = tx.to_screen_x(slot.cx);
    let sy = tx.to_screen_y(slot.cy);
    let sw = tx.len(slot.width);
    let sh = tx.len(slot.height);
    let inner_w = (sw - 10.0).max(8.0);
    let binding_text = binding.to_string();
    let empty = binding.is_empty();

    let primary_color = if is_selected {
        palette.label_accent
    } else if empty {
        palette.label_muted
    } else {
        palette.label_primary
    };
    let tag_color = if is_selected {
        palette.label_accent
    } else {
        palette.label_muted
    };

    if matches!(slot.shape, ElementShape::RockerRight) {
        let arc_cx = sx - sw * 0.08;
        let arc_w = (sw * 0.72).max(10.0);
        draw_centered_text(
            cr,
            arc_cx,
            sy - sh * 0.16,
            arc_w,
            slot.short_label,
            TextStyle {
                size: (tx.scale * 8.0).clamp(8.0, 10.5),
                bold: false,
                color: tag_color,
            },
        );
        draw_centered_text(
            cr,
            arc_cx,
            sy + sh * 0.13,
            arc_w,
            if empty { "—" } else { &binding_text },
            TextStyle {
                size: (tx.scale * 9.5).clamp(8.5, 11.5),
                bold: true,
                color: primary_color,
            },
        );
    } else if sh >= 42.0 && sw >= 42.0 {
        draw_centered_text(
            cr,
            sx,
            sy - sh * 0.24,
            inner_w,
            slot.short_label,
            TextStyle {
                size: (tx.scale * 8.0).clamp(8.0, 10.5),
                bold: false,
                color: tag_color,
            },
        );
        draw_centered_text(
            cr,
            sx,
            sy + sh * 0.10,
            inner_w,
            if empty { "Unassigned" } else { &binding_text },
            TextStyle {
                size: (tx.scale * 10.5).clamp(9.0, 13.0),
                bold: true,
                color: primary_color,
            },
        );
    } else if empty {
        draw_centered_text(
            cr,
            sx,
            sy,
            inner_w,
            slot.short_label,
            TextStyle {
                size: (tx.scale * 8.8).clamp(8.5, 11.5),
                bold: true,
                color: tag_color,
            },
        );
    } else {
        let compact = format!("{}·{}", slot.short_label, binding_text);
        let font_size = (tx.scale * 8.2).clamp(8.0, 10.5);
        cr.select_font_face(
            ui_font_family(),
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(font_size);
        let fits_compact = cr
            .text_extents(&compact)
            .is_ok_and(|ext| ext.width() <= inner_w + 4.0);
        let label = if fits_compact {
            compact.as_str()
        } else {
            &binding_text
        };
        draw_centered_text(
            cr,
            sx,
            sy,
            inner_w + 2.0,
            label,
            TextStyle {
                size: font_size,
                bold: true,
                color: primary_color,
            },
        );
    }
}

// -----------------------------------------------------------------------------
// Lighting View Live Vector Preview & Simulation
// -----------------------------------------------------------------------------

/// Evaluated visual output for a single LED channel at a point in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LedVisualSample {
    pub color: RgbColor,
    /// Visual brightness/envelope intensity in `0.0..=1.0`.
    pub intensity: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct LightingSimSnapshot {
    pub mode: LightMode,
    pub speed: LightSpeed,
    pub has_brightness: bool,
    pub is_static_only: bool,
    pub full_speed_off: bool,
    pub held_key_idx: Option<usize>,
    pub key_press_starts_us: [i64; KEY_SLOTS_CAP],
    pub toggle_on: bool,
    pub animation_origin_us: i64,
}

impl Default for LightingSimSnapshot {
    fn default() -> Self {
        Self {
            mode: LightMode::RainbowColor,
            speed: LightSpeed::default(),
            has_brightness: true,
            is_static_only: false,
            full_speed_off: false,
            held_key_idx: None,
            key_press_starts_us: [-10_000_000; KEY_SLOTS_CAP],
            toggle_on: true,
            animation_origin_us: 0,
        }
    }
}

pub fn fade_duration_secs(fade_delay: u16) -> f64 {
    (fade_delay as f64 / 2560.0 * 0.65).clamp(0.12, 3.0)
}

pub fn rainbow_period_secs(rainbow_delay: u16) -> f64 {
    (rainbow_delay as f64 / 2560.0 * 3.2).clamp(0.35, 12.0)
}

/// Convert normalized hue `[0.0, 1.0)`, saturation, and value `[0.0, 1.0]` to [`RgbColor`].
pub fn hsv_to_rgb(hue: f64, sat: f64, val: f64) -> RgbColor {
    let h = hue.rem_euclid(1.0) * 6.0;
    let i = h.floor() as u8;
    let f = h - (i as f64);
    let p = val * (1.0 - sat);
    let q = val * (1.0 - f * sat);
    let t = val * (1.0 - (1.0 - f) * sat);

    let (r, g, b) = match i % 6 {
        0 => (val, t, p),
        1 => (q, val, p),
        2 => (p, val, t),
        3 => (p, q, val),
        4 => (t, p, val),
        _ => (val, p, q),
    };

    RgbColor::new(
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn channel_last_pulse_us(led_index: usize, led_count: usize, snap: &LightingSimSnapshot) -> i64 {
    let mut best = snap
        .key_press_starts_us
        .get(led_index)
        .copied()
        .unwrap_or(-10_000_000);
    // Secondary keys (BT3..BT8) pulse all LED zones simultaneously
    for &ts in snap.key_press_starts_us.iter().skip(led_count) {
        best = best.max(ts);
    }
    best
}

fn channel_is_held(led_index: usize, led_count: usize, snap: &LightingSimSnapshot) -> bool {
    match snap.held_key_idx {
        Some(k) if k == led_index => true,
        Some(k) if k >= led_count => true,
        _ => false,
    }
}

/// Pure function computing live visual color + intensity for an LED channel at `now_us`.
pub fn evaluate_lighting_preview(
    led_index: usize,
    led_count: usize,
    leds: &[LedConfig],
    snap: &LightingSimSnapshot,
    now_us: i64,
) -> LedVisualSample {
    let cfg = leds.get(led_index).copied().unwrap_or_default();
    if snap.full_speed_off {
        return LedVisualSample {
            color: cfg.color,
            intensity: 0.0,
        };
    }

    let brightness_scale = if snap.has_brightness {
        cfg.brightness.effective_level() as f64 / 4.0
    } else {
        1.0
    };

    if snap.is_static_only {
        return LedVisualSample {
            color: cfg.color,
            intensity: brightness_scale,
        };
    }

    let elapsed_s = (now_us - snap.animation_origin_us).max(0) as f64 / 1_000_000.0;
    let rainbow_period = rainbow_period_secs(snap.speed.rainbow_delay);
    let hue = (elapsed_s / rainbow_period + (led_index as f64) * 0.14).rem_euclid(1.0);
    let rainbow_rgb = hsv_to_rgb(hue, 1.0, 1.0);

    let fade_secs = fade_duration_secs(snap.speed.fade_delay);
    let held = channel_is_held(led_index, led_count, snap);
    let pulse_age_s =
        (now_us - channel_last_pulse_us(led_index, led_count, snap)).max(0) as f64 / 1_000_000.0;

    let ease_out_factor = if held {
        1.0
    } else if pulse_age_s < fade_secs {
        let p = (pulse_age_s / fade_secs).clamp(0.0, 1.0);
        (1.0 - p) * (1.0 - p)
    } else {
        0.0
    };

    let ease_in_factor = if held {
        0.12
    } else if pulse_age_s < fade_secs {
        let p = (pulse_age_s / fade_secs).clamp(0.0, 1.0);
        0.12 + 0.88 * (p * p)
    } else {
        1.0
    };

    match snap.mode {
        LightMode::KeepOff => LedVisualSample {
            color: cfg.color,
            intensity: 0.0,
        },
        LightMode::KeepOn => LedVisualSample {
            color: cfg.color,
            intensity: brightness_scale,
        },
        LightMode::EaseOut => LedVisualSample {
            color: cfg.color,
            intensity: brightness_scale * ease_out_factor,
        },
        LightMode::EaseIn => LedVisualSample {
            color: cfg.color,
            intensity: brightness_scale * ease_in_factor,
        },
        LightMode::KeyPress => {
            let active = held || pulse_age_s < 0.28;
            LedVisualSample {
                color: cfg.color,
                intensity: if active { brightness_scale } else { 0.0 },
            }
        }
        LightMode::KeySwitch => LedVisualSample {
            color: cfg.color,
            intensity: if snap.toggle_on {
                brightness_scale
            } else {
                0.0
            },
        },
        LightMode::RainbowColor => LedVisualSample {
            color: rainbow_rgb,
            intensity: brightness_scale,
        },
        LightMode::RainbowEaseOut => LedVisualSample {
            color: rainbow_rgb,
            intensity: brightness_scale * ease_out_factor,
        },
        LightMode::RainbowSwitch => LedVisualSample {
            color: rainbow_rgb,
            intensity: if snap.toggle_on {
                brightness_scale
            } else {
                0.0
            },
        },
    }
}

struct LightingPreviewState {
    model: DeviceModel,
    capabilities: DeviceCapabilities,
    leds: Vec<LedConfig>,
    full_speed_mode: bool,
    is_page_visible: bool,
    hovered_idx: Option<usize>,
    sim: LightingSimSnapshot,
    demo_next_beat_us: i64,
    demo_cursor: usize,
    last_manual_us: i64,
    last_draw_us: i64,
    tick_id: Option<gtk::TickCallbackId>,
}

impl LightingPreviewState {
    fn needs_animation(&self) -> bool {
        if !self.is_page_visible
            || self.model.is_bootloader()
            || self.capabilities.led_count == 0
            || self.leds.is_empty()
            || (self.capabilities.has_full_speed && self.full_speed_mode)
            || self.capabilities.supported_light_modes.is_empty()
        {
            return false;
        }
        !matches!(self.sim.mode, LightMode::KeepOn | LightMode::KeepOff)
    }

    fn is_trigger_mode(&self) -> bool {
        !self.capabilities.supported_light_modes.is_empty()
            && matches!(
                self.sim.mode,
                LightMode::EaseOut
                    | LightMode::EaseIn
                    | LightMode::KeyPress
                    | LightMode::KeySwitch
                    | LightMode::RainbowEaseOut
                    | LightMode::RainbowSwitch
            )
    }

    fn demo_interval_us(&self) -> i64 {
        match self.sim.mode {
            LightMode::KeyPress => 850_000,
            LightMode::KeySwitch | LightMode::RainbowSwitch => 1_150_000,
            _ => {
                let fade_us = (fade_duration_secs(self.sim.speed.fade_delay) * 1_000_000.0) as i64;
                (fade_us + 320_000).clamp(480_000, 3_600_000)
            }
        }
    }

    fn advance_demo_if_due(&mut self, now_us: i64) {
        if !self.is_trigger_mode() || self.sim.held_key_idx.is_some() {
            return;
        }
        if now_us - self.last_manual_us < MANUAL_DEMO_COOLDOWN_US {
            return;
        }
        if now_us < self.demo_next_beat_us {
            return;
        }

        let count = self.model.key_count().max(1);
        let key_idx = self.demo_cursor % count;
        self.demo_cursor = self.demo_cursor.wrapping_add(1);

        if key_idx < KEY_SLOTS_CAP {
            self.sim.key_press_starts_us[key_idx] = now_us;
        }
        if matches!(
            self.sim.mode,
            LightMode::KeySwitch | LightMode::RainbowSwitch
        ) {
            self.sim.toggle_on = !self.sim.toggle_on;
        }
        self.demo_next_beat_us = now_us + self.demo_interval_us();
    }
}

/// Interactive Cairo lighting simulation matching each [`DeviceModel`]'s LED zones.
pub struct LightingDiagramPreview {
    area: gtk::DrawingArea,
    state: Rc<RefCell<LightingPreviewState>>,
}

impl Default for LightingDiagramPreview {
    fn default() -> Self {
        Self::new()
    }
}

impl LightingDiagramPreview {
    pub fn new() -> Self {
        let now = glib::monotonic_time();
        let area = gtk::DrawingArea::builder()
            .hexpand(true)
            .vexpand(false)
            .content_width(270)
            .content_height(176)
            .build();

        let state = Rc::new(RefCell::new(LightingPreviewState {
            model: DeviceModel::SimpadNanoYearEdition,
            capabilities: DeviceModel::SimpadNanoYearEdition.capabilities(),
            leds: Vec::new(),
            full_speed_mode: false,
            is_page_visible: false,
            hovered_idx: None,
            sim: LightingSimSnapshot {
                animation_origin_us: now,
                ..Default::default()
            },
            demo_next_beat_us: now,
            demo_cursor: 0,
            last_manual_us: -10_000_000,
            last_draw_us: 0,
            tick_id: None,
        }));

        let state_draw = Rc::clone(&state);
        area.set_draw_func(move |_area, cr, width, height| {
            let st = state_draw.borrow();
            draw_lighting_diagram(cr, width as f64, height as f64, &st, glib::monotonic_time());
        });

        // Allow clicking or holding preview keycaps to test reactive effects immediately
        let click = gtk::GestureClick::builder().button(1).build();
        let state_press = Rc::clone(&state);
        click.connect_pressed(move |gesture, _, x, y| {
            let Some(widget) = gesture.widget() else {
                return;
            };
            let redraw = {
                let mut st = state_press.borrow_mut();
                if let Some(idx) = hit_test_widget_key(&widget, st.model, x, y) {
                    let now = glib::monotonic_time();
                    st.sim.held_key_idx = Some(idx);
                    if idx < KEY_SLOTS_CAP {
                        st.sim.key_press_starts_us[idx] = now;
                    }
                    st.last_manual_us = now;
                    if matches!(st.sim.mode, LightMode::KeySwitch | LightMode::RainbowSwitch) {
                        st.sim.toggle_on = !st.sim.toggle_on;
                    }
                    true
                } else {
                    false
                }
            };
            if redraw {
                widget.queue_draw();
            }
        });

        let state_release = Rc::clone(&state);
        click.connect_released(move |gesture, _, _, _| {
            release_held_key(&state_release, gesture.widget().as_ref());
        });

        let state_cancel = Rc::clone(&state);
        click.connect_cancel(move |gesture, _| {
            release_held_key(&state_cancel, gesture.widget().as_ref());
        });
        area.add_controller(click);

        // Hover pointer affordance over clickable keys (only updates on hover transition)
        let motion = gtk::EventControllerMotion::new();
        let state_motion = Rc::clone(&state);
        motion.connect_motion(move |ctrl, x, y| {
            let Some(widget) = ctrl.widget() else {
                return;
            };
            let transition = {
                let mut st = state_motion.borrow_mut();
                let hit = hit_test_widget_key(&widget, st.model, x, y);
                if st.hovered_idx != hit {
                    st.hovered_idx = hit;
                    Some(hit)
                } else {
                    None
                }
            };
            if let Some(hit) = transition {
                widget.set_cursor_from_name(hit.map(|_| "pointer"));
            }
        });
        let state_motion_leave = Rc::clone(&state);
        motion.connect_leave(move |ctrl| {
            let Some(widget) = ctrl.widget() else {
                return;
            };
            let had_hover = state_motion_leave.borrow_mut().hovered_idx.take().is_some();
            if had_hover {
                widget.set_cursor_from_name(None);
            }
        });
        area.add_controller(motion);

        connect_theme_redraw(&area);

        Self { area, state }
    }

    pub fn widget(&self) -> &gtk::DrawingArea {
        &self.area
    }

    pub fn sync(
        &self,
        model: DeviceModel,
        capabilities: DeviceCapabilities,
        mode: LightMode,
        leds: &[LedConfig],
        speed: LightSpeed,
        full_speed_mode: bool,
    ) {
        let needs_redraw = {
            let mut st = self.state.borrow_mut();
            let now = glib::monotonic_time();
            let mode_changed = st.sim.mode != mode;
            let model_changed = st.model != model;
            let changed = model_changed
                || st.capabilities != capabilities
                || mode_changed
                || st.leds.as_slice() != leds
                || st.sim.speed != speed
                || st.full_speed_mode != full_speed_mode;

            st.model = model;
            st.capabilities = capabilities;
            st.full_speed_mode = full_speed_mode;
            st.leds.clear();
            st.leds.extend_from_slice(leds);

            st.sim.mode = mode;
            st.sim.speed = speed;
            st.sim.has_brightness = capabilities.has_brightness;
            st.sim.is_static_only = capabilities.supported_light_modes.is_empty();
            st.sim.full_speed_off = capabilities.has_full_speed && full_speed_mode;

            if mode_changed || model_changed {
                st.sim.toggle_on = true;
                st.demo_next_beat_us = now;
            }
            changed
        };

        self.sync_tick_callback();
        if needs_redraw {
            self.area.queue_draw();
        }
    }

    pub fn set_page_active(&self, active: bool) {
        let changed = {
            let mut st = self.state.borrow_mut();
            let diff = st.is_page_visible != active;
            st.is_page_visible = active;
            if active {
                st.demo_next_beat_us = glib::monotonic_time();
            } else {
                st.sim.held_key_idx = None;
            }
            diff
        };
        self.sync_tick_callback();
        if changed && active {
            self.area.queue_draw();
        }
    }

    fn sync_tick_callback(&self) {
        let (should_animate, removed_tick) = {
            let mut st = self.state.borrow_mut();
            let want = st.needs_animation();
            let to_remove = if !want { st.tick_id.take() } else { None };
            let already_running = st.tick_id.is_some();
            (want && !already_running, to_remove)
        };

        if let Some(id) = removed_tick {
            id.remove();
        }

        if should_animate {
            let weak_state = Rc::downgrade(&self.state);
            let tick_id = self.area.add_tick_callback(move |area, _clock| {
                let Some(state_rc) = weak_state.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                let should_draw = {
                    let mut st = state_rc.borrow_mut();
                    if !st.needs_animation() {
                        st.tick_id.take();
                        return glib::ControlFlow::Break;
                    }
                    let now = glib::monotonic_time();
                    st.advance_demo_if_due(now);
                    if now - st.last_draw_us >= FRAME_INTERVAL_US {
                        st.last_draw_us = now;
                        true
                    } else {
                        false
                    }
                };
                if should_draw {
                    area.queue_draw();
                }
                glib::ControlFlow::Continue
            });

            self.state.borrow_mut().tick_id = Some(tick_id);
        }
    }
}

fn release_held_key(state: &Rc<RefCell<LightingPreviewState>>, widget: Option<&gtk::Widget>) {
    let redraw = {
        let mut st = state.borrow_mut();
        if let Some(idx) = st.sim.held_key_idx.take() {
            let now = glib::monotonic_time();
            if idx < KEY_SLOTS_CAP {
                st.sim.key_press_starts_us[idx] = now;
            }
            st.last_manual_us = now;
            true
        } else {
            false
        }
    };
    if redraw && let Some(w) = widget {
        w.queue_draw();
    }
}

fn draw_lighting_diagram(
    cr: &cairo::Context,
    canvas_w: f64,
    canvas_h: f64,
    st: &LightingPreviewState,
    now_us: i64,
) {
    draw_lighting_diagram_with_palette(
        cr,
        canvas_w,
        canvas_h,
        st,
        now_us,
        PreviewPalette::for_theme(is_dark_theme()),
    );
}

fn draw_lighting_diagram_with_palette(
    cr: &cairo::Context,
    canvas_w: f64,
    canvas_h: f64,
    st: &LightingPreviewState,
    now_us: i64,
    palette: PreviewPalette,
) {
    let layout = st.model.layout();
    if layout.leds.is_empty() {
        return;
    }

    let tx = CanvasTransform::fit(canvas_w, canvas_h, layout, 14.0);

    // Darker matte housing makes RGB glow and colors legible in both light & dark themes
    draw_chassis_and_usb(
        cr,
        tx,
        layout,
        palette.lighting_chassis_fill,
        palette.chassis_border,
        palette,
    );

    // Pre-compute each channel sample once per draw pass
    let mut channel_samples = [LedVisualSample {
        color: RgbColor::default(),
        intensity: 0.0,
    }; 4];
    for (idx, sample) in channel_samples
        .iter_mut()
        .enumerate()
        .take(st.capabilities.led_count.min(4))
    {
        *sample =
            evaluate_lighting_preview(idx, st.capabilities.led_count, &st.leds, &st.sim, now_us);
    }

    // Clip LED radial halos cleanly inside chassis shell
    let _ = cr.save();
    rounded_rect_path(
        cr,
        tx.to_screen_x(0.0) + 1.0,
        tx.to_screen_y(0.0) + 1.0,
        tx.len(layout.body_width) - 2.0,
        tx.len(layout.body_height) - 2.0,
        (tx.len(layout.corner_radius) - 1.0).max(4.0),
    );
    cr.clip();

    for zone in layout.leds {
        let sample = channel_samples
            .get(zone.led_index)
            .copied()
            .unwrap_or(LedVisualSample {
                color: RgbColor::default(),
                intensity: 0.0,
            });
        draw_led_zone(cr, tx, zone, sample, palette);
    }
    let _ = cr.restore();

    // Overlay frosted switch/key silhouettes on top so key positions match the physical pad
    for slot in layout.keys {
        if slot.show_rocker_arrows {
            draw_rocker_arrows(cr, tx, slot.cx, slot.cy, slot.height, palette.usb_muted);
        }

        let is_pressed = st.sim.held_key_idx == Some(slot.index);
        build_element_path(cr, tx, ElementGeom::from_key(slot));
        palette.lighting_key_tint.apply(cr);
        let _ = cr.fill_preserve();

        if is_pressed {
            Rgba::new(1.0, 1.0, 1.0, 0.85).apply(cr);
            cr.set_line_width(1.8);
        } else {
            Rgba::new(1.0, 1.0, 1.0, 0.24).apply(cr);
            cr.set_line_width(1.0);
        }
        let _ = cr.stroke();
    }

    // Subtle zone tags (LED1/LED2 or G1/G2) centered over primary LEDs
    for zone in layout.leds {
        if zone.primary {
            let sx = tx.to_screen_x(zone.cx);
            let sy = tx.to_screen_y(zone.cy);
            let sample = channel_samples
                .get(zone.led_index)
                .copied()
                .unwrap_or(LedVisualSample {
                    color: RgbColor::default(),
                    intensity: 0.0,
                });
            let lum = (0.299 * sample.color.r as f64
                + 0.587 * sample.color.g as f64
                + 0.114 * sample.color.b as f64)
                / 255.0;
            let text_color = if sample.intensity > 0.55 && lum > 0.55 {
                Rgba::new(0.08, 0.09, 0.11, 0.92)
            } else {
                Rgba::new(0.96, 0.97, 0.99, 0.90)
            };
            draw_centered_text(
                cr,
                sx,
                sy,
                tx.len(zone.width) - 8.0,
                zone.label,
                TextStyle {
                    size: (tx.scale * 8.5).clamp(8.0, 11.0),
                    bold: true,
                    color: text_color,
                },
            );
        }
    }
}

fn draw_led_zone(
    cr: &cairo::Context,
    tx: CanvasTransform,
    zone: &crate::device::LedZoneLayout,
    sample: LedVisualSample,
    palette: PreviewPalette,
) {
    let sx = tx.to_screen_x(zone.cx);
    let sy = tx.to_screen_y(zone.cy);
    let sw = tx.len(zone.width);
    let sh = tx.len(zone.height);
    let intensity = sample.intensity.clamp(0.0, 1.0);

    // Outer ambient halo glow
    if intensity > 0.02 {
        let base_r = sw.max(sh) * 0.5;
        let outer_r = base_r * if zone.primary { 1.65 } else { 1.85 };
        let halo = cairo::RadialGradient::new(sx, sy, base_r * 0.15, sx, sy, outer_r);
        let c = sample.color;
        let rf = c.r as f64 / 255.0;
        let gf = c.g as f64 / 255.0;
        let bf = c.b as f64 / 255.0;
        halo.add_color_stop_rgba(0.0, rf, gf, bf, 0.72 * intensity);
        halo.add_color_stop_rgba(0.50, rf, gf, bf, 0.28 * intensity);
        halo.add_color_stop_rgba(1.0, rf, gf, bf, 0.0);

        let _ = cr.set_source(&halo);
        cr.new_path();
        cr.arc(sx, sy, outer_r, 0.0, std::f64::consts::TAU);
        let _ = cr.fill();
    }

    // Core diffuser element
    build_element_path(cr, tx, ElementGeom::from_led(zone));
    palette.led_off_fill.apply(cr);
    let _ = cr.fill_preserve();

    if intensity > 0.01 {
        Rgba::from_rgb(sample.color, 0.88 * intensity).apply(cr);
        let _ = cr.fill_preserve();
    }

    let rim = if intensity > 0.05 {
        Rgba::from_rgb(sample.color, 0.45 + 0.5 * intensity)
    } else {
        Rgba::new(0.45, 0.48, 0.54, 0.45)
    };
    rim.apply(cr);
    cr.set_line_width(1.1);
    let _ = cr.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::lighting::Brightness;

    #[test]
    fn test_hsv_to_rgb_cardinals() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), RgbColor::new(255, 0, 0));
        assert_eq!(hsv_to_rgb(1.0 / 3.0, 1.0, 1.0), RgbColor::new(0, 255, 0));
        assert_eq!(hsv_to_rgb(2.0 / 3.0, 1.0, 1.0), RgbColor::new(0, 0, 255));
    }

    #[test]
    fn test_lighting_preview_evaluation_modes() {
        let leds = [
            LedConfig::new(RgbColor::new(255, 0, 128), Brightness::new(2)),
            LedConfig::new(RgbColor::new(0, 255, 64), Brightness::new(4)),
        ];
        let mut snap = LightingSimSnapshot {
            mode: LightMode::KeepOn,
            has_brightness: true,
            ..Default::default()
        };

        let s0 = evaluate_lighting_preview(0, 2, &leds, &snap, 1_000_000);
        assert_eq!(s0.color, RgbColor::new(255, 0, 128));
        assert!((s0.intensity - 0.5).abs() < 1e-6);

        let s1 = evaluate_lighting_preview(1, 2, &leds, &snap, 1_000_000);
        assert!((s1.intensity - 1.0).abs() < 1e-6);

        snap.mode = LightMode::KeepOff;
        assert_eq!(
            evaluate_lighting_preview(0, 2, &leds, &snap, 1_000_000).intensity,
            0.0
        );

        snap.mode = LightMode::KeepOn;
        snap.full_speed_off = true;
        assert_eq!(
            evaluate_lighting_preview(1, 2, &leds, &snap, 1_000_000).intensity,
            0.0
        );

        // EaseOut immediately after press vs after fade duration
        snap.full_speed_off = false;
        snap.mode = LightMode::EaseOut;
        snap.key_press_starts_us[0] = 1_000_000;
        let peak = evaluate_lighting_preview(0, 2, &leds, &snap, 1_000_000).intensity;
        let mid = evaluate_lighting_preview(0, 2, &leds, &snap, 1_300_000).intensity;
        let end = evaluate_lighting_preview(0, 2, &leds, &snap, 5_000_000).intensity;
        assert!(peak > mid && mid > end);
        assert_eq!(end, 0.0);

        // EaseIn dips cleanly then recovers
        snap.mode = LightMode::EaseIn;
        let dip = evaluate_lighting_preview(0, 2, &leds, &snap, 1_000_000).intensity;
        let recovered = evaluate_lighting_preview(0, 2, &leds, &snap, 5_000_000).intensity;
        assert!(dip < recovered);
        assert!((recovered - 0.5).abs() < 1e-6);

        // Rainbow changes color over time
        snap.mode = LightMode::RainbowColor;
        let r0 = evaluate_lighting_preview(0, 2, &leds, &snap, 0).color;
        let r1 = evaluate_lighting_preview(0, 2, &leds, &snap, 1_000_000).color;
        assert_ne!(r0, r1);
    }

    #[test]
    fn test_cairo_render_all_models_and_themes_without_errors() {
        use crate::config::device_settings::DeviceSettings;

        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 320, 200)
            .expect("Cairo ImageSurface");
        let cr = cairo::Context::new(&surface).expect("Cairo Context");

        let mut models = DeviceModel::all_models().to_vec();
        models.push(DeviceModel::Bootloader);

        for dark in [false, true] {
            let palette = PreviewPalette::for_theme(dark);
            for &model in &models {
                let settings = DeviceSettings::default_for_model(model);
                let keys_state = KeysDiagramState {
                    model,
                    keys: settings.keys.clone(),
                    selected_idx: 0,
                    hovered_idx: Some(0),
                };
                draw_keys_diagram_with_palette(&cr, 320.0, 200.0, &keys_state, palette);
                assert_eq!(cr.status(), Ok(()));

                let mut light_state = LightingPreviewState {
                    model,
                    capabilities: model.capabilities(),
                    leds: settings.leds.clone(),
                    full_speed_mode: false,
                    is_page_visible: true,
                    hovered_idx: None,
                    sim: LightingSimSnapshot {
                        mode: settings.light_mode,
                        has_brightness: model.capabilities().has_brightness,
                        is_static_only: model.capabilities().supported_light_modes.is_empty(),
                        ..Default::default()
                    },
                    demo_next_beat_us: 0,
                    demo_cursor: 0,
                    last_manual_us: -10_000_000,
                    last_draw_us: 0,
                    tick_id: None,
                };
                draw_lighting_diagram_with_palette(
                    &cr,
                    320.0,
                    200.0,
                    &light_state,
                    500_000,
                    palette,
                );
                assert_eq!(cr.status(), Ok(()));

                // Also verify full-speed-off state renders cleanly
                light_state.full_speed_mode = true;
                light_state.sim.full_speed_off = true;
                draw_lighting_diagram_with_palette(
                    &cr,
                    320.0,
                    200.0,
                    &light_state,
                    500_000,
                    palette,
                );
                assert_eq!(cr.status(), Ok(()));
            }
        }
    }
}
