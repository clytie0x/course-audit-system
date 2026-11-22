//! Shared Tailwind class strings so cards, tags and buttons match everywhere.

pub const CARD: &str =
    "rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-soft";
pub const TAG: &str =
    "inline-flex items-center gap-1 text-xs font-medium rounded-full px-2.5 py-px whitespace-nowrap";
pub const TAG_OK: &str = "bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400";
pub const TAG_ACCENT: &str = "bg-accent-50 text-accent-700 dark:bg-accent-900 dark:text-accent-300";
pub const BUTTON: &str = "inline-flex items-center gap-2 h-9 px-3.5 rounded-[9px] border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 text-[13.5px] font-medium text-zinc-800 dark:text-zinc-200 shadow-soft hover:bg-zinc-50 dark:hover:bg-zinc-800 whitespace-nowrap focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-950";
pub const TAG_WARN: &str = "bg-amber-50 text-amber-700 dark:bg-amber-950 dark:text-amber-400";
pub const TAG_MUTED: &str = "bg-zinc-50 text-zinc-600 border border-zinc-200 dark:bg-zinc-800 dark:text-zinc-300 dark:border-zinc-700";
pub const SECTION_TITLE: &str =
    "text-base font-semibold leading-snug text-zinc-900 dark:text-zinc-100";

/// Grade badge: green for A, amber while in progress, red for F/E, neutral otherwise.
pub fn grade_class(grade: &str) -> &'static str {
    match grade.trim().to_ascii_uppercase().as_str() {
        "A" => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md text-[12.5px] font-semibold bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400",
        "IP" => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md text-[12.5px] font-semibold bg-amber-50 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
        "F" | "E" => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md text-[12.5px] font-semibold bg-red-50 text-red-700 dark:bg-red-950 dark:text-red-400",
        _ => "inline-grid place-items-center min-w-[30px] h-6 px-1.5 rounded-md border text-[12.5px] font-semibold bg-zinc-50 text-zinc-700 border-zinc-200 dark:bg-zinc-800 dark:text-zinc-300 dark:border-zinc-700",
    }
}
