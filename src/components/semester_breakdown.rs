//! Semesters tab: one card per term with its courses, credits and GPA.

use crate::components::status::{by_semester, fmt_credits};
use crate::components::ui::{grade_class, CARD, TAG, TAG_ACCENT, TAG_WARN};
use audit_core::models::compute_gpa;
use audit_core::models::AuditResult;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn SemesterBreakdown(result: AuditResult) -> impl IntoView {
    let AppState { is_thai, .. } = expect_context::<AppState>();
    let terms = by_semester(&result);

    if terms.is_empty() {
        return view! {
            <p class=format!("{CARD} px-4 py-5 text-sm text-zinc-500")>
                {move || if is_thai.get() { "ใบแสดงผลการเรียนนี้ไม่มีข้อมูลภาคการศึกษา" } else { "This transcript has no semester labels" }}
            </p>
        }
        .into_any();
    }

    view! {
        <div class="grid gap-3.5 grid-cols-1 md:grid-cols-2 xl:grid-cols-3">
            {terms.into_iter().map(|(sem, courses)| {
                let in_progress = courses.iter().any(|c| c.grade.eq_ignore_ascii_case("IP"));
                let credits: f32 = courses.iter().filter(|c| !c.grade.eq_ignore_ascii_case("IP")).map(|c| c.credit).sum();
                let gpa = compute_gpa(&courses);
                let sem_label = sem.clone();
                view! {
                    <section class=format!("{CARD} overflow-hidden")>
                        <div class="flex items-baseline gap-2 px-4 pt-3.5 pb-2.5">
                            <h3 class="font-semibold text-[15px] text-zinc-900">
                                {move || if is_thai.get() { format!("ภาค {sem_label}") } else { format!("Semester {sem_label}") }}
                            </h3>
                            <span class="flex-1"></span>
                            <span class="text-[13px] text-zinc-500 tabular-nums">
                                {move || if is_thai.get() { format!("{} นก.", fmt_credits(credits)) } else { format!("{} cr", fmt_credits(credits)) }}
                            </span>
                            {if in_progress {
                                view! { <span class=format!("{TAG} {TAG_WARN}")>{move || if is_thai.get() { "กำลังเรียน" } else { "In progress" }}</span> }.into_any()
                            } else {
                                gpa.map(|g| view! { <span class=format!("{TAG} {TAG_ACCENT} tabular-nums")>{format!("{g:.2}")}</span> }).into_any()
                            }}
                        </div>
                        {courses.into_iter().map(|c| view! {
                            <div class="grid grid-cols-[66px_minmax(0,1fr)_22px_34px] gap-2 items-center px-4 py-1.5 border-t border-zinc-200 text-[13.5px]">
                                <span class="font-mono text-[0.86em] text-zinc-500">{c.code.clone()}</span>
                                <span class="truncate text-zinc-800" title=c.name.clone()>{c.name.clone()}</span>
                                <span class="text-zinc-500 tabular-nums text-right">{fmt_credits(c.credit)}</span>
                                <span class=grade_class(&c.grade)>{c.grade.clone()}</span>
                            </div>
                        }).collect::<Vec<_>>()}
                    </section>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
