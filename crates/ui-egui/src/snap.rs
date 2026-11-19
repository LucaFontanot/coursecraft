//! Smart-guide snapping for canvas moves and resizes.
//!
//! The maths is pure: callers pass slide-space rectangles and the snap
//! threshold already converted through the slide transform. The returned
//! offset is added to the moving rectangle, and the guides are drawn in screen
//! space by the canvas.

use egui::{Rect, Vec2};

/// A guide line to draw while the moving rectangle is snapped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapGuide {
    /// True for a vertical line (`at` is an x coordinate), false for horizontal.
    pub vertical: bool,
    /// The slide-space coordinate of the line.
    pub at: f32,
    /// Start of the relevant span along the other axis, in slide space.
    pub start: f32,
    /// End of the relevant span along the other axis, in slide space.
    pub end: f32,
}

/// The offset to apply and the guides to draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapResult {
    pub offset: Vec2,
    pub guides: Vec<SnapGuide>,
}

fn finite_rect(rect: Rect) -> bool {
    [rect.min.x, rect.min.y, rect.max.x, rect.max.y]
        .iter()
        .all(|value| value.is_finite())
}

fn better(
    current: Option<(f32, f32, f32, f32, f32)>,
    candidate: (f32, f32, f32, f32, f32),
) -> Option<(f32, f32, f32, f32, f32)> {
    match current {
        Some(existing) if existing.0 <= candidate.0 => Some(existing),
        _ => Some(candidate),
    }
}

/// Snap `moving` to the nearest of `targets` plus `slide`, independently on
/// each axis. Each rectangle contributes its left/centre/right and
/// top/middle/bottom lines. A candidate closer than `threshold` (slide units)
/// wins; ties keep the earlier target. Non-finite input is ignored.
#[must_use]
pub fn snap_rect(moving: Rect, targets: &[Rect], slide: Rect, threshold: f32) -> SnapResult {
    let mut result = SnapResult::default();
    if !finite_rect(moving) || !threshold.is_finite() || threshold <= 0.0 {
        return result;
    }

    let moving_x = [moving.left(), moving.center().x, moving.right()];
    let moving_y = [moving.top(), moving.center().y, moving.bottom()];

    let mut best_x: Option<(f32, f32, f32, f32, f32)> = None;
    let mut best_y: Option<(f32, f32, f32, f32, f32)> = None;

    for target in targets.iter().chain(std::iter::once(&slide)) {
        if !finite_rect(*target) {
            continue;
        }
        let target_x = [target.left(), target.center().x, target.right()];
        let target_y = [target.top(), target.center().y, target.bottom()];

        let span_top = moving.top().min(target.top());
        let span_bottom = moving.bottom().max(target.bottom());
        let span_left = moving.left().min(target.left());
        let span_right = moving.right().max(target.right());

        for line in target_x {
            for edge in moving_x {
                let delta = line - edge;
                let distance = delta.abs();
                if distance <= threshold {
                    best_x = better(best_x, (distance, delta, line, span_top, span_bottom));
                }
            }
        }
        for line in target_y {
            for edge in moving_y {
                let delta = line - edge;
                let distance = delta.abs();
                if distance <= threshold {
                    best_y = better(best_y, (distance, delta, line, span_left, span_right));
                }
            }
        }
    }

    if let Some((_, delta, line, start, end)) = best_x {
        result.offset.x = delta;
        result.guides.push(SnapGuide {
            vertical: true,
            at: line,
            start,
            end,
        });
    }
    if let Some((_, delta, line, start, end)) = best_y {
        result.offset.y = delta;
        result.guides.push(SnapGuide {
            vertical: false,
            at: line,
            start,
            end,
        });
    }
    result
}

/// Extra snap lines: user guides and a grid, in slide pixels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineSnaps {
    /// x coordinates of vertical lines.
    pub vertical: Vec<f32>,
    /// y coordinates of horizontal lines.
    pub horizontal: Vec<f32>,
    /// Grid spacing, when snapping to the grid.
    pub grid: Option<f32>,
}

impl LineSnaps {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertical.is_empty() && self.horizontal.is_empty() && self.grid.is_none()
    }
}

/// Snapping is off while Cmd/Ctrl is held.
#[must_use]
pub fn snapping_active(enabled: bool, command_held: bool) -> bool {
    enabled && !command_held
}

/// The closest line for one axis: `(distance, delta, line)`.
fn nearest_line(
    edges: [f32; 3],
    lines: &[f32],
    grid: Option<f32>,
    threshold: f32,
) -> Option<(f32, f32, f32)> {
    let mut best: Option<(f32, f32, f32)> = None;
    let mut consider = |line: f32, edge: f32| {
        let delta = line - edge;
        let distance = delta.abs();
        if line.is_finite()
            && distance <= threshold
            && best.is_none_or(|(current, _, _)| distance < current)
        {
            best = Some((distance, delta, line));
        }
    };
    for line in lines {
        for edge in edges {
            consider(*line, edge);
        }
    }
    if let Some(step) = grid
        && step.is_finite()
        && step > 0.0
    {
        // The leading and trailing edges follow the grid; the centre does not.
        for edge in [edges[0], edges[2]] {
            consider((edge / step).round() * step, edge);
        }
    }
    best
}

/// Like [`snap_rect`] but also snapping to guides and the grid. `smart` turns
/// the object-to-object guides on. A line closer than the smart candidate wins.
#[must_use]
pub fn snap_rect_lines(
    moving: Rect,
    targets: &[Rect],
    slide: Rect,
    threshold: f32,
    smart: bool,
    lines: &LineSnaps,
) -> SnapResult {
    let mut result = if smart {
        snap_rect(moving, targets, slide, threshold)
    } else {
        SnapResult::default()
    };
    if !finite_rect(moving) || !threshold.is_finite() || threshold <= 0.0 || lines.is_empty() {
        return result;
    }
    let xs = [moving.left(), moving.center().x, moving.right()];
    let ys = [moving.top(), moving.center().y, moving.bottom()];
    let smart_x = result.guides.iter().any(|g| g.vertical);
    let smart_y = result.guides.iter().any(|g| !g.vertical);
    if let Some((distance, delta, line)) = nearest_line(xs, &lines.vertical, lines.grid, threshold)
        && (!smart_x || distance < result.offset.x.abs())
    {
        result.guides.retain(|g| !g.vertical);
        result.offset.x = delta;
        result.guides.push(SnapGuide {
            vertical: true,
            at: line,
            start: moving.top(),
            end: moving.bottom(),
        });
    }
    if let Some((distance, delta, line)) =
        nearest_line(ys, &lines.horizontal, lines.grid, threshold)
        && (!smart_y || distance < result.offset.y.abs())
    {
        result.guides.retain(|g| g.vertical);
        result.offset.y = delta;
        result.guides.push(SnapGuide {
            vertical: false,
            at: line,
            start: moving.left(),
            end: moving.right(),
        });
    }
    result
}

/// An equal-gap guide to draw while the moving rectangle is spacing-snapped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingGuide {
    /// True for a vertical gap (between objects in a column, `from..to` is y
    /// and `across` is x); false for a horizontal gap (`from..to` is x and
    /// `across` is y).
    pub vertical: bool,
    /// Start of the gap along its axis, in slide space.
    pub from: f32,
    /// End of the gap along its axis, in slide space.
    pub to: f32,
    /// Coordinate on the perpendicular axis where the guide sits, in slide space.
    pub across: f32,
}

/// The offset to apply and the equal-spacing guides to draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpacingResult {
    pub offset: Vec2,
    pub guides: Vec<SpacingGuide>,
}

#[derive(Debug, Clone, Copy)]
struct AxisSpan {
    min: f32,
    max: f32,
    perp_min: f32,
    perp_max: f32,
}

impl AxisSpan {
    fn from_rect(rect: Rect, vertical: bool) -> Self {
        if vertical {
            Self {
                min: rect.top(),
                max: rect.bottom(),
                perp_min: rect.left(),
                perp_max: rect.right(),
            }
        } else {
            Self {
                min: rect.left(),
                max: rect.right(),
                perp_min: rect.top(),
                perp_max: rect.bottom(),
            }
        }
    }

    fn center(self) -> f32 {
        (self.min + self.max) * 0.5
    }

    fn len(self) -> f32 {
        self.max - self.min
    }
}

fn perp_overlap(a: AxisSpan, b: AxisSpan) -> Option<(f32, f32)> {
    let start = a.perp_min.max(b.perp_min);
    let end = a.perp_max.min(b.perp_max);
    (start < end).then_some((start, end))
}

#[derive(Debug, Clone, Copy)]
struct ExistingGap {
    gap: f32,
    from: f32,
    to: f32,
    across: f32,
}

fn existing_gaps_on_axis(spans: &[AxisSpan]) -> Vec<ExistingGap> {
    let mut gaps = Vec::new();
    for (i, &a) in spans.iter().enumerate() {
        for (j, &b) in spans.iter().enumerate() {
            if i == j || a.max >= b.min {
                continue;
            }
            let Some((p0, p1)) = perp_overlap(a, b) else {
                continue;
            };
            let blocked = spans.iter().enumerate().any(|(k, &c)| {
                k != i
                    && k != j
                    && c.max > a.max
                    && c.min < b.min
                    && c.perp_max > p0
                    && c.perp_min < p1
            });
            if !blocked {
                gaps.push(ExistingGap {
                    gap: b.min - a.max,
                    from: a.max,
                    to: b.min,
                    across: (p0 + p1) * 0.5,
                });
            }
        }
    }
    gaps
}

fn push_unique_guide(guides: &mut Vec<SpacingGuide>, guide: SpacingGuide) {
    let duplicate = guides.iter().any(|existing| {
        existing.vertical == guide.vertical
            && (existing.from - guide.from).abs() < 0.01
            && (existing.to - guide.to).abs() < 0.01
            && (existing.across - guide.across).abs() < 0.01
    });
    if !duplicate {
        guides.push(guide);
    }
}

fn snap_spacing_axis(
    moving: Rect,
    targets: &[Rect],
    threshold: f32,
    vertical: bool,
) -> Option<(f32, Vec<SpacingGuide>)> {
    let m = AxisSpan::from_rect(moving, vertical);
    let spans: Vec<AxisSpan> = targets
        .iter()
        .map(|r| AxisSpan::from_rect(*r, vertical))
        .collect();
    let existing = existing_gaps_on_axis(&spans);

    // Immediate neighbours in the same row/column as `moving`.
    let mut prev: Option<(AxisSpan, (f32, f32))> = None;
    let mut next: Option<(AxisSpan, (f32, f32))> = None;
    for &t in &spans {
        let Some(perp) = perp_overlap(m, t) else {
            continue;
        };
        if t.min < m.min && t.max <= m.min + threshold && t.center() < m.center() {
            let replace = prev.is_none_or(|(cur, _)| t.max > cur.max);
            if replace {
                prev = Some((t, perp));
            }
        }
        if t.max > m.max && t.min >= m.max - threshold && t.center() > m.center() {
            let replace = next.is_none_or(|(cur, _)| t.min < cur.min);
            if replace {
                next = Some((t, perp));
            }
        }
    }

    let mut best_distance = f32::INFINITY;
    let mut best_delta = 0.0_f32;
    let mut candidates: Vec<(f32, SpacingGuide)> = Vec::new();
    let mut consider = |delta: f32, guides: &[SpacingGuide]| {
        let distance = delta.abs();
        if distance > threshold {
            return;
        }
        if distance + 0.001 < best_distance {
            best_distance = distance;
            best_delta = delta;
        }
        for &guide in guides {
            candidates.push((delta, guide));
        }
    };

    // 1. Equal gap on both sides when moving between `prev` and `next`.
    if let (Some((p, p_perp)), Some((n, n_perp))) = (prev, next) {
        let space = n.min - p.max;
        let len = m.len();
        if space > len {
            let gap = (space - len) * 0.5;
            let delta = (p.max + gap) - m.min;
            let g1 = SpacingGuide {
                vertical,
                from: p.max,
                to: p.max + gap,
                across: (p_perp.0 + p_perp.1) * 0.5,
            };
            let g2 = SpacingGuide {
                vertical,
                from: n.min - gap,
                to: n.min,
                across: (n_perp.0 + n_perp.1) * 0.5,
            };
            consider(delta, &[g1, g2]);
        }
    }

    // 2. Gap to `prev` matching an existing gap.
    if let Some((p, p_perp)) = prev {
        for eg in &existing {
            let target_min = p.max + eg.gap;
            if let Some((n, _)) = next
                && target_min + m.len() >= n.min
            {
                continue;
            }
            let delta = target_min - m.min;
            let g_existing = SpacingGuide {
                vertical,
                from: eg.from,
                to: eg.to,
                across: eg.across,
            };
            let g_new = SpacingGuide {
                vertical,
                from: p.max,
                to: target_min,
                across: (p_perp.0 + p_perp.1) * 0.5,
            };
            consider(delta, &[g_existing, g_new]);
        }
    }

    // 3. Gap to `next` matching an existing gap.
    if let Some((n, n_perp)) = next {
        for eg in &existing {
            let target_max = n.min - eg.gap;
            if let Some((p, _)) = prev
                && target_max - m.len() <= p.max
            {
                continue;
            }
            let delta = target_max - m.max;
            let g_existing = SpacingGuide {
                vertical,
                from: eg.from,
                to: eg.to,
                across: eg.across,
            };
            let g_new = SpacingGuide {
                vertical,
                from: target_max,
                to: n.min,
                across: (n_perp.0 + n_perp.1) * 0.5,
            };
            consider(delta, &[g_existing, g_new]);
        }
    }

    if !best_distance.is_finite() {
        return None;
    }

    let mut matched = Vec::new();
    for (delta, guide) in candidates {
        if (delta - best_delta).abs() < 0.01 {
            push_unique_guide(&mut matched, guide);
        }
    }
    Some((best_delta, matched))
}

/// Snap `moving` so its gap to a neighbour in the same row or column matches
/// an existing gap between two other `targets` on that axis, or matches the
/// gap on its opposite side. Non-finite inputs are ignored.
#[must_use]
pub fn snap_spacing(moving: Rect, targets: &[Rect], threshold: f32) -> SpacingResult {
    let mut result = SpacingResult::default();
    if !finite_rect(moving) || !threshold.is_finite() || threshold <= 0.0 {
        return result;
    }
    let valid_targets: Vec<Rect> = targets
        .iter()
        .copied()
        .filter(|r| finite_rect(*r) && r.width() > 0.0 && r.height() > 0.0)
        .collect();
    if valid_targets.is_empty() {
        return result;
    }

    if let Some((dx, guides_x)) = snap_spacing_axis(moving, &valid_targets, threshold, false) {
        result.offset.x = dx;
        result.guides.extend(guides_x);
    }
    if let Some((dy, guides_y)) = snap_spacing_axis(moving, &valid_targets, threshold, true) {
        result.offset.y = dy;
        result.guides.extend(guides_y);
    }
    result
}

/// Combine an edge/centre [`SnapResult`] with a [`SpacingResult`], keeping
/// edge/centre snapping on any axis where it already matched and falling back
/// to equal-spacing on the remaining axes.
#[must_use]
pub fn apply_spacing_fallback(snap: &mut SnapResult, spacing: SpacingResult) -> Vec<SpacingGuide> {
    let snapped_x = snap.guides.iter().any(|g| g.vertical);
    let snapped_y = snap.guides.iter().any(|g| !g.vertical);
    let mut guides = Vec::new();
    if !snapped_x && spacing.guides.iter().any(|g| !g.vertical) {
        snap.offset.x = spacing.offset.x;
        guides.extend(spacing.guides.iter().copied().filter(|g| !g.vertical));
    }
    if !snapped_y && spacing.guides.iter().any(|g| g.vertical) {
        snap.offset.y = spacing.offset.y;
        guides.extend(spacing.guides.iter().copied().filter(|g| g.vertical));
    }
    guides
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    const SLIDE: Rect = Rect {
        min: pos2(0.0, 0.0),
        max: pos2(1000.0, 1000.0),
    };

    #[test]
    fn snaps_each_horizontal_edge() {
        let target = rect(100.0, 100.0, 100.0, 100.0);
        let moving = rect(103.0, 300.0, 50.0, 50.0);
        // Left edge of moving is 3 px from target's left.
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.x, -3.0);
        assert_eq!(result.guides.len(), 1);
        assert!(result.guides[0].vertical);

        // Centre to centre.
        let moving = rect(123.0, 300.0, 50.0, 50.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.x, 2.0);

        // Right to right.
        let moving = rect(153.0, 300.0, 50.0, 50.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.x, -3.0);
    }

    #[test]
    fn snaps_each_vertical_edge() {
        let target = rect(100.0, 100.0, 100.0, 100.0);
        let moving = rect(300.0, 103.0, 50.0, 50.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.y, -3.0);
        assert!(!result.guides[0].vertical);

        let moving = rect(300.0, 123.0, 50.0, 50.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.y, 2.0);

        let moving = rect(300.0, 153.0, 50.0, 50.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.y, -3.0);
    }

    #[test]
    fn threshold_boundary_is_inclusive_and_just_outside_is_ignored() {
        let target = rect(100.0, 100.0, 100.0, 100.0);
        let at_threshold = rect(106.0, 300.0, 10.0, 10.0);
        assert_eq!(
            snap_rect(at_threshold, &[target], SLIDE, 6.0).offset.x,
            -6.0
        );

        let outside = rect(106.1, 300.0, 10.0, 10.0);
        let result = snap_rect(outside, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.x, 0.0);
        assert!(result.guides.is_empty());
    }

    #[test]
    fn nearest_target_wins_per_axis() {
        let near = rect(102.0, 100.0, 100.0, 10.0);
        let far = rect(105.0, 200.0, 100.0, 10.0);
        let moving = rect(100.0, 300.0, 50.0, 10.0);
        let result = snap_rect(moving, &[far, near], SLIDE, 6.0);
        assert_eq!(result.offset.x, 2.0);
    }

    #[test]
    fn snaps_both_axes_independently() {
        let target = rect(100.0, 100.0, 100.0, 100.0);
        let moving = rect(103.0, 197.0, 50.0, 50.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result.offset.x, -3.0);
        assert_eq!(result.offset.y, 3.0);
        assert_eq!(result.guides.len(), 2);
    }

    #[test]
    fn returns_no_offset_when_nothing_is_close() {
        let target = rect(100.0, 100.0, 10.0, 10.0);
        let moving = rect(300.0, 300.0, 10.0, 10.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result, SnapResult::default());
    }

    #[test]
    fn ignores_non_finite_input() {
        let target = rect(f32::NAN, 100.0, 10.0, 10.0);
        let moving = rect(300.0, 300.0, 10.0, 10.0);
        let result = snap_rect(moving, &[target], SLIDE, 6.0);
        assert_eq!(result, SnapResult::default());

        let moving = rect(f32::INFINITY, 300.0, 10.0, 10.0);
        let result = snap_rect(moving, &[], SLIDE, 6.0);
        assert_eq!(result, SnapResult::default());

        let result = snap_rect(rect(300.0, 300.0, 10.0, 10.0), &[], SLIDE, f32::NAN);
        assert_eq!(result, SnapResult::default());
    }

    #[test]
    fn snaps_to_the_slide_edges() {
        let moving = rect(4.0, 4.0, 50.0, 50.0);
        let result = snap_rect(moving, &[], SLIDE, 6.0);
        assert_eq!(result.offset.x, -4.0);
        assert_eq!(result.offset.y, -4.0);
        assert_eq!(result.guides.len(), 2);
    }

    #[test]
    fn snaps_to_a_guide_and_the_grid_and_prefers_the_nearer_line() {
        let lines = LineSnaps {
            vertical: vec![400.0],
            horizontal: vec![],
            grid: None,
        };
        let moving = rect(297.0, 300.0, 100.0, 50.0);
        let result = snap_rect_lines(moving, &[], SLIDE, 6.0, false, &lines);
        assert_eq!(result.offset.x, 3.0, "right edge to the guide at 400");
        assert_eq!(result.offset.y, 0.0);

        let grid = LineSnaps {
            vertical: vec![],
            horizontal: vec![],
            grid: Some(20.0),
        };
        let moving = rect(123.0, 258.0, 50.0, 50.0);
        let result = snap_rect_lines(moving, &[], SLIDE, 6.0, false, &grid);
        assert_eq!(result.offset.x, -3.0);
        assert_eq!(result.offset.y, 2.0);

        // A guide 1px away beats a smart guide 3px away.
        let target = rect(103.0, 600.0, 10.0, 10.0);
        let lines = LineSnaps {
            vertical: vec![101.0],
            horizontal: vec![],
            grid: None,
        };
        let moving = rect(100.0, 300.0, 50.0, 50.0);
        let result = snap_rect_lines(moving, &[target], SLIDE, 6.0, true, &lines);
        assert_eq!(result.offset.x, 1.0);
        assert_eq!(result.guides.iter().filter(|g| g.vertical).count(), 1);
    }

    #[test]
    fn line_snapping_is_off_with_the_modifier_and_ignores_bad_input() {
        assert!(snapping_active(true, false));
        assert!(!snapping_active(true, true));
        assert!(!snapping_active(false, false));
        let lines = LineSnaps {
            vertical: vec![f32::NAN, 100.0],
            horizontal: vec![],
            grid: Some(f32::NAN),
        };
        let result = snap_rect_lines(rect(98.0, 0.0, 10.0, 10.0), &[], SLIDE, 6.0, false, &lines);
        assert_eq!(result.offset.x, 2.0);
        let bad = snap_rect_lines(
            rect(f32::NAN, 0.0, 1.0, 1.0),
            &[],
            SLIDE,
            6.0,
            false,
            &lines,
        );
        assert_eq!(bad, SnapResult::default());
    }

    #[test]
    fn three_boxes_in_a_row_with_equal_gaps_snap() {
        // Middle box between left (100..150) and right (250..300):
        // available space is 100, moving width is 50, so equal gap is 25 at x = 175.
        let left = rect(100.0, 100.0, 50.0, 40.0);
        let right = rect(250.0, 100.0, 50.0, 40.0);
        let moving = rect(178.0, 100.0, 50.0, 40.0);
        let result = snap_spacing(moving, &[left, right], 6.0);
        assert_eq!(result.offset.x, -3.0);
        assert_eq!(result.offset.y, 0.0);
        assert_eq!(result.guides.len(), 2);
        assert!(result.guides.iter().all(|g| !g.vertical));
        assert_eq!(result.guides[0].from, 150.0);
        assert_eq!(result.guides[0].to, 175.0);
        assert_eq!(result.guides[1].from, 225.0);
        assert_eq!(result.guides[1].to, 250.0);

        // Third box beside two existing boxes (gap 24): snaps to 24 px right of the second.
        let a = rect(100.0, 100.0, 50.0, 40.0);
        let b = rect(174.0, 100.0, 50.0, 40.0);
        let moving = rect(245.0, 100.0, 50.0, 40.0); // currently 21 px from b, 3 px from 24
        let result = snap_spacing(moving, &[a, b], 6.0);
        assert_eq!(result.offset.x, 3.0);
        assert_eq!(result.guides.len(), 2);
    }

    #[test]
    fn unequal_gaps_beyond_threshold_do_not_snap() {
        let a = rect(100.0, 100.0, 50.0, 40.0);
        let b = rect(174.0, 100.0, 50.0, 40.0); // gap 24
        let moving = rect(260.0, 100.0, 50.0, 40.0); // gap 36 (12 px off, > threshold 6)
        let result = snap_spacing(moving, &[a, b], 6.0);
        assert_eq!(result, SpacingResult::default());
    }

    #[test]
    fn vertical_columns_snap_equal_gaps() {
        let top = rect(100.0, 100.0, 60.0, 40.0);
        let bottom = rect(100.0, 240.0, 60.0, 40.0);
        // Available space 100, moving height 40 => equal gap 30 at y = 170.
        let moving = rect(100.0, 166.0, 60.0, 40.0);
        let result = snap_spacing(moving, &[top, bottom], 6.0);
        assert_eq!(result.offset.y, 4.0);
        assert_eq!(result.offset.x, 0.0);
        assert_eq!(result.guides.len(), 2);
        assert!(result.guides.iter().all(|g| g.vertical));
        assert_eq!(result.guides[0].from, 140.0);
        assert_eq!(result.guides[0].to, 170.0);
        assert_eq!(result.guides[1].from, 210.0);
        assert_eq!(result.guides[1].to, 240.0);
    }

    #[test]
    fn spacing_ignores_non_finite_rects() {
        let good_left = rect(100.0, 100.0, 50.0, 40.0);
        let good_right = rect(250.0, 100.0, 50.0, 40.0);
        let bad_target = rect(f32::NAN, 100.0, 50.0, 40.0);
        let moving = rect(178.0, 100.0, 50.0, 40.0);
        let result = snap_spacing(moving, &[bad_target, good_left, good_right], 6.0);
        assert_eq!(result.offset.x, -3.0);

        let bad_moving = rect(f32::INFINITY, 100.0, 50.0, 40.0);
        assert_eq!(
            snap_spacing(bad_moving, &[good_left, good_right], 6.0),
            SpacingResult::default()
        );
    }

    #[test]
    fn edge_snap_beats_spacing_snap_and_ties_prefer_smaller_movement() {
        // Edge target aligns moving.left to 179 (4 px away), while spacing wants 175 (0 px away).
        // Edge/centre snapping still wins when both apply within threshold.
        let left = rect(100.0, 100.0, 50.0, 40.0);
        let right = rect(250.0, 100.0, 50.0, 40.0);
        let edge_target = rect(179.0, 300.0, 50.0, 40.0);
        let moving = rect(175.0, 100.0, 50.0, 40.0);
        let targets = [left, right, edge_target];

        let mut snap = snap_rect(moving, &targets, SLIDE, 6.0);
        let spacing = snap_spacing(moving, &targets, 6.0);
        let spacing_guides = apply_spacing_fallback(&mut snap, spacing);
        assert_eq!(snap.offset.x, 4.0);
        assert!(spacing_guides.is_empty());

        // Among spacing candidates, smaller movement wins.
        let a = rect(0.0, 100.0, 40.0, 40.0);
        let b = rect(60.0, 100.0, 40.0, 40.0); // gap 20
        let c = rect(125.0, 100.0, 40.0, 40.0); // gap 25
        // Next to c: gap 20 puts moving at 185, gap 25 puts moving at 190.
        // At x = 189, gap 25 is 1 px away and gap 20 is 4 px away => 1 px wins.
        let moving = rect(189.0, 100.0, 40.0, 40.0);
        let res = snap_spacing(moving, &[a, b, c], 6.0);
        assert_eq!(res.offset.x, 1.0);
    }
}
