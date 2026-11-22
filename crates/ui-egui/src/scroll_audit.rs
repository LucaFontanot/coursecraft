//! Scroll-area viewport tracking for the headless nested-scroll checker (`UX-02`).

#[cfg(test)]
use egui::{Context, Id, LayerId, Rect};

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollRecord {
    pub layer: LayerId,
    pub rect: Rect,
    pub vertical: bool,
    pub from_dialog_body: bool,
}

#[cfg(test)]
fn scrolls_id() -> Id {
    Id::unique("coursecraft_scroll_audit")
}

/// Record a scroll viewport `rect` on `layer` during this pass.
#[cfg(test)]
pub(crate) fn record_scroll(
    ctx: &Context,
    layer: LayerId,
    rect: Rect,
    vertical: bool,
    from_dialog_body: bool,
) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let slot =
            d.get_temp_mut_or_insert_with(scrolls_id(), || (pass, Vec::<ScrollRecord>::new()));
        if slot.0 != pass {
            slot.0 = pass;
            slot.1.clear();
        }
        slot.1.push(ScrollRecord {
            layer,
            rect,
            vertical,
            from_dialog_body,
        });
    });
}

/// The scroll viewports recorded during the current pass.
#[cfg(test)]
pub(crate) fn recorded_scrolls(ctx: &Context) -> Vec<ScrollRecord> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| {
        d.get_temp::<(u64, Vec<ScrollRecord>)>(scrolls_id())
            .filter(|(p, _)| *p == pass)
            .map(|(_, v)| v)
            .unwrap_or_default()
    })
}

/// The scroll viewports recorded during the current or just-completed pass.
#[cfg(test)]
pub(crate) fn last_recorded_scrolls(ctx: &Context) -> Vec<ScrollRecord> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| {
        d.get_temp::<(u64, Vec<ScrollRecord>)>(scrolls_id())
            .filter(|(p, _)| *p == pass || p.saturating_add(1) == pass)
            .map(|(_, v)| v)
            .unwrap_or_default()
    })
}
