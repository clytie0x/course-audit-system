//! Shared Tailwind class strings so cards, tags and buttons match everywhere.

pub const CARD: &str = "rounded-xl border border-zinc-200 bg-white shadow-soft";
pub const TAG: &str =
    "inline-flex items-center gap-1 text-xs font-medium rounded-full px-2.5 py-px whitespace-nowrap";
pub const TAG_OK: &str = "bg-emerald-50 text-emerald-700";
pub const TAG_ACCENT: &str = "bg-accent-50 text-accent-700";
pub const BUTTON: &str = "inline-flex items-center gap-2 h-9 px-3.5 rounded-[9px] border border-zinc-200 bg-white text-[13.5px] font-medium text-zinc-800 shadow-soft hover:bg-zinc-50 whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 focus-visible:ring-offset-2";
pub const TAG_WARN: &str = "bg-amber-50 text-amber-700";
pub const TAG_MUTED: &str = "bg-zinc-50 text-zinc-600 border border-zinc-200";
pub const SECTION_TITLE: &str = "text-base font-semibold leading-snug text-zinc-900";

/// Grade badge: green for A, amber while in progress, red for F/E, neutral otherwise.
pub fn grade_class(grade: &str) -> &'static str {
    match grade.trim().to_ascii_uppercase().as_str() {
        "A" => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md text-[12.5px] font-semibold bg-emerald-50 text-emerald-700",
        "IP" => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md text-[12.5px] font-semibold bg-amber-50 text-amber-700",
        "F" | "E" => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md text-[12.5px] font-semibold bg-red-50 text-red-700",
        _ => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md border text-[12.5px] font-semibold bg-zinc-50 text-zinc-700 border-zinc-200",
    }
}
