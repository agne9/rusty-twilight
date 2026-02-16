//! Pure pagination math and page-window shaping helpers.

/// Compute the number of pages for a paginated list.
pub fn total_pages(item_count: usize, per_page: usize) -> usize {
    item_count.div_ceil(per_page.max(1))
}

/// Clamp a requested page into a valid range.
pub fn clamp_page(page: usize, total_pages: usize) -> usize {
    page.clamp(1, total_pages.max(1))
}

/// Resolve a modal-entered page using the modal's total-pages hint.
///
/// The hint can become stale if data changed after the modal opened.
/// This function safely bounds the target page to both the current total
/// and the hint range seen by the user.
pub fn resolve_modal_target_page(
    entered_page: usize,
    current_total_pages: usize,
    hinted_total_pages: usize,
) -> usize {
    let max_allowed_page = std::cmp::min(current_total_pages, std::cmp::max(hinted_total_pages, 1));
    clamp_page(entered_page, max_allowed_page)
}

/// Return start/end indices for a page window.
pub fn page_window(total_items: usize, per_page: usize, page: usize) -> (usize, usize) {
    let safe_per_page = per_page.max(1);
    let start = page.saturating_sub(1).saturating_mul(safe_per_page);
    let end = (start + safe_per_page).min(total_items);
    (start.min(total_items), end)
}

/// Parse a one-based page argument.
///
/// Returns `Some(page)` when the value is valid (`>= 1`), otherwise `None`.
pub fn parse_one_based_page(raw: Option<&str>) -> Option<usize> {
    match raw {
        Some(value) => value.parse::<usize>().ok().filter(|page| *page >= 1),
        None => Some(1),
    }
}

/// Build a bullet-list description string for a specific page window.
pub fn paginated_bulleted_description(items: &[String], per_page: usize, page: usize) -> String {
    let total = total_pages(items.len(), per_page);
    let page = clamp_page(page, total);
    let (start, end) = page_window(items.len(), per_page, page);
    format!("- {}", items[start..end].join("\n- "))
}
