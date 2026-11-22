//! Per-semester accordion with semester GPA and credits.

use audit_core::models::compute_gpa;
use audit_core::models::AuditResult;
use audit_core::models::Course;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn SemesterBreakdown(result: AuditResult) -> impl IntoView {
    let AppState { is_thai, .. } = expect_context::<AppState>();

    view! {
        // ── Semester-by-semester breakdown ──────
        {
            // Gather every attempt (incl. failed ones) that has a
            // semester tag and group them by semester label. Results
            // saved before `attempts` existed fall back to categories.
            let source: Vec<&Course> = if result.attempts.is_empty() {
                result.categories.iter().flat_map(|c| c.courses.iter()).collect()
            } else {
                result.attempts.iter().collect()
            };
            let mut by_sem: Vec<(String, Vec<Course>)> = Vec::new();
            for course in source {
                if let Some(sem) = &course.semester {
                    if let Some(bucket) = by_sem.iter_mut().find(|(s, _)| s == sem) {
                        bucket.1.push(course.clone());
                    } else {
                        by_sem.push((sem.clone(), vec![course.clone()]));
                    }
                }
            }
            // Sort by year then by term (e.g., 1/2567 before 2/2567 before 1/2568)
            by_sem.sort_by(|(a, _), (b, _)| {
                let parse = |s: &str| -> (u32, u32) {
                    let mut parts = s.split('/');
                    let term = parts.next().and_then(|t| t.parse::<u32>().ok()).unwrap_or(0);
                    let year = parts.next().and_then(|y| y.parse::<u32>().ok()).unwrap_or(0);
                    (year, term)
                };
                parse(a).cmp(&parse(b))
            });
            (!by_sem.is_empty()).then(move || view! {
                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft overflow-hidden">
                    <div class="px-5 py-4 border-b border-zinc-100 dark:border-zinc-800 flex items-center gap-2.5">
                        <svg class="w-4 h-4 text-zinc-400 dark:text-zinc-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5"/></svg>
                        <h3 class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "รายวิชาแยกตามภาคการศึกษา" } else { "Courses by Semester" }}</h3>
                    </div>
                    <div class="divide-y divide-zinc-100 dark:divide-zinc-800">
                        {by_sem.into_iter().map(|(sem, bucket)| {
                            let total_cr: f32 = bucket.iter()
                                .filter(|c| !c.grade.eq_ignore_ascii_case("IP"))
                                .map(|c| c.credit).sum();
                            let sem_gpa = compute_gpa(&bucket);
                            let bucket_render = bucket.clone();
                            view! {
                                <details class="group">
                                    <summary class="list-none px-5 py-3.5 flex items-center justify-between cursor-pointer hover:bg-zinc-50/80 dark:hover:bg-zinc-800/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-inset">
                                        <div class="flex items-center gap-3 min-w-0">
                                            <svg class="w-4 h-4 text-zinc-400 dark:text-zinc-500 transition-transform duration-200 group-open:rotate-90" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5"/></svg>
                                            <span class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { format!("ภาคการศึกษา {}", sem) } else { format!("Semester {}", sem) }}</span>
                                            <span class="text-2xs text-zinc-400 dark:text-zinc-500 font-mono">{bucket_render.len()}" วิชา"</span>
                                        </div>
                                        <div class="flex items-center gap-3 shrink-0">
                                            {sem_gpa.map(|g| view! {
                                                <span class="text-2xs font-semibold text-brand-600 dark:text-brand-400 bg-brand-50 dark:bg-brand-950/40 px-2 py-0.5 rounded">{format!("GPA {:.2}", g)}</span>
                                            })}
                                            <span class="text-xs font-mono font-medium text-zinc-500 dark:text-zinc-400 tabular-nums">{format!("{:.0} cr", total_cr)}</span>
                                        </div>
                                    </summary>
                                    <div class="divide-y divide-zinc-100/80 dark:divide-zinc-800/80 bg-zinc-50/40 dark:bg-zinc-900/60 border-t border-zinc-100 dark:border-zinc-800">
                                        {bucket_render.iter().map(|course| {
                                            let is_ip = course.grade.eq_ignore_ascii_case("IP");
                                            let grade_color = if is_ip {
                                                "bg-amber-50 text-amber-700 border-amber-200/60 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-900/50"
                                            } else {
                                                match course.grade.chars().next().unwrap_or('F') {
                                                    'A' => "bg-emerald-50 text-emerald-700 border-emerald-200/60 dark:bg-emerald-950/40 dark:text-emerald-300 dark:border-emerald-900/50",
                                                    'B' => "bg-blue-50 text-blue-700 border-blue-200/60 dark:bg-blue-950/40 dark:text-blue-300 dark:border-blue-900/50",
                                                    'C' => "bg-amber-50 text-amber-700 border-amber-200/60 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-900/50",
                                                    'D' => "bg-orange-50 text-orange-700 border-orange-200/60 dark:bg-orange-950/40 dark:text-orange-300 dark:border-orange-900/50",
                                                    _ => "bg-zinc-50 text-zinc-600 border-zinc-200 dark:bg-zinc-800/60 dark:text-zinc-300 dark:border-zinc-700",
                                                }
                                            };
                                            let credit_cell = if is_ip { "—".to_string() } else { format!("{}", course.credit as u32) };
                                            view! {
                                                <div class="flex items-center justify-between px-5 py-2.5">
                                                    <div class="flex items-center gap-3 min-w-0 flex-1">
                                                        <span class="font-mono text-2xs font-semibold text-zinc-400 dark:text-zinc-500 w-14 shrink-0">{course.code.clone()}</span>
                                                        <span class="text-[13px] text-zinc-700 dark:text-zinc-300 truncate">{course.name.clone()}</span>
                                                    </div>
                                                    <div class="flex items-center gap-2.5 shrink-0 ml-3">
                                                        <span class={format!("text-2xs font-bold min-w-[1.75rem] h-5 px-1 flex items-center justify-center rounded border {}", grade_color)}>
                                                            {course.grade.clone()}
                                                        </span>
                                                        <span class="text-2xs text-zinc-400 dark:text-zinc-500 font-mono w-6 text-right">{credit_cell}</span>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </details>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })
        }
    }
}
