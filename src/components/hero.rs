//! Total credits earned and overall GPA.

use audit_core::models::AuditResult;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn HeroCard(result: AuditResult) -> impl IntoView {
    let AppState { is_thai, .. } = expect_context::<AppState>();

    view! {
        // ── Hero: Total Credits ─────────────────
        <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft p-6 sm:p-8 relative overflow-hidden">
            <div class="relative flex flex-col sm:flex-row sm:items-end sm:justify-between gap-4">
                <div>
                    <p class="text-xs font-semibold text-brand-600 dark:text-brand-400 uppercase tracking-widest mb-1">{move || if is_thai.get() { "ความคืบหน้าทั้งหมด" } else { "Total Progress" }}</p>
                    <div class="flex items-baseline gap-2">
                        <span class="text-5xl sm:text-6xl font-extrabold tracking-tighter text-zinc-900 dark:text-zinc-100 tabular-nums">
                            {result.total_credits as u32}
                        </span>
                        <span class="text-base font-medium text-zinc-400 dark:text-zinc-500">{move || if is_thai.get() { "หน่วยกิตที่ได้รับ" } else { "credits earned" }}</span>
                    </div>
                </div>
                <div class="flex flex-col items-end gap-2 self-start sm:self-auto">
                    {result.gpa.map(|gpa| view! {
                        <div class="flex items-baseline gap-2 bg-brand-50 dark:bg-brand-950/40 text-brand-700 dark:text-brand-300 rounded-lg px-3 py-1.5">
                            <span class="text-2xs font-semibold uppercase tracking-wider opacity-70">{move || if is_thai.get() { "เกรดเฉลี่ย" } else { "GPA" }}</span>
                            <span class="text-lg font-extrabold tabular-nums">{format!("{:.2}", gpa)}</span>
                        </div>
                    })}
                    <div class="flex items-center gap-1.5 text-xs text-zinc-500 dark:text-zinc-400 bg-zinc-50 dark:bg-zinc-800/60 rounded-lg px-3 py-1.5">
                        <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4.26 10.147a60.436 60.436 0 00-.491 6.347A48.627 48.627 0 0112 20.904a48.627 48.627 0 018.232-4.41 60.46 60.46 0 00-.491-6.347"/></svg>
                        <span class="font-medium">{move || if is_thai.get() { "วท.บ. (วิทยาการคอมพิวเตอร์)" } else { "B.Sc. (Computer Science)" }}</span>
                    </div>
                </div>
            </div>
        </div>
    }
}
