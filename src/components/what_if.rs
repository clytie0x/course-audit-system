//! What-If simulator: add hypothetical courses to the audit input.

use audit_core::auditor::unmet_prerequisites;
use audit_core::data::curriculum;
use audit_core::models::ParsedCourse;
use audit_core::parser::normalize_course_code;
use leptos::prelude::*;

use crate::app::AppState;

#[component]
pub fn WhatIfSimulator() -> impl IntoView {
    let AppState {
        parsed_courses,
        plan_result,
        curriculum_id,
        planned_courses,
        set_planned_courses,
        plan_code,
        set_plan_code,
        plan_name,
        set_plan_name,
        plan_credit,
        set_plan_credit,
        plan_grade,
        set_plan_grade,
        is_thai,
        ..
    } = expect_context::<AppState>();
    // Append a hypothetical course to the plan and let the effect above
    // re-audit. No-ops if the form is empty.
    let add_planned = move |_: web_sys::MouseEvent| {
        let code = plan_code.get_untracked().trim().to_string();
        if code.is_empty() {
            return;
        }
        let name = plan_name.get_untracked().trim().to_string();
        let credit = plan_credit.get_untracked().max(0.0);
        let grade = plan_grade.get_untracked().trim().to_uppercase();
        let planned = ParsedCourse {
            code: normalize_course_code(&code),
            name: if name.is_empty() { code.clone() } else { name },
            grade,
            parsed_credit: credit,
            semester: Some(String::from("PLAN")),
            tag: None,
        };
        set_planned_courses.update(|v| v.push(planned));
        set_plan_code.set(String::new());
        set_plan_name.set(String::new());
    };

    let remove_planned = move |idx: usize| {
        set_planned_courses.update(|v| {
            if idx < v.len() {
                v.remove(idx);
            }
        });
    };

    let clear_plan = move |_: web_sys::MouseEvent| {
        set_planned_courses.set(Vec::new());
    };

    view! {
        // ── What-If Simulator ───────────────────
        <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft overflow-hidden print:hidden">
            <div class="px-5 py-4 border-b border-zinc-100 dark:border-zinc-800 flex items-center gap-2.5">
                <svg class="w-4 h-4 text-brand-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M9.75 3.104v5.714a2.25 2.25 0 01-.659 1.591L5 14.5M9.75 3.104c-.251.023-.501.05-.75.082m.75-.082a24.301 24.301 0 014.5 0m0 0v5.714c0 .597.237 1.17.659 1.591L19.8 15.3M14.25 3.104c.251.023.501.05.75.082M19.8 15.3l-1.57.393A9.065 9.065 0 0112 15a9.065 9.065 0 00-6.23-.693L5 14.5m14.8.8l1.402 1.402c1.232 1.232.65 3.378-1.067 3.81A48.26 48.26 0 0112 21.75c-2.568 0-5.053-.201-7.135-.539-1.718-.432-2.3-2.578-1.068-3.81L5 14.5"/></svg>
                <h3 class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "จำลองแผน (What-If)" } else { "What-If Simulator" }}</h3>
                <span class="text-2xs text-zinc-500 dark:text-zinc-400 ml-1">{move || if is_thai.get() { "ใส่วิชาสมมติเพื่อดูผลกระทบ" } else { "Add hypothetical courses to see projected impact" }}</span>
            </div>
            <div class="p-5 space-y-4">
                // Input form
                <div class="grid grid-cols-1 sm:grid-cols-[1fr_2fr_auto_auto_auto] gap-2.5">
                    <input
                        type="text"
                        placeholder={move || if is_thai.get() { "รหัส เช่น 344-301" } else { "Code e.g. 344-301" }}
                        class="min-w-0 text-[13px] font-mono px-3 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                        aria-label={move || if is_thai.get() { "รหัสวิชา" } else { "Course code" }}
                        prop:value=move || plan_code.get()
                        on:input=move |ev| set_plan_code.set(event_target_value(&ev))
                    />
                    <input
                        type="text"
                        placeholder={move || if is_thai.get() { "ชื่อวิชา (ไม่จำเป็น)" } else { "Course name (optional)" }}
                        class="min-w-0 text-[13px] px-3 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                        aria-label={move || if is_thai.get() { "ชื่อวิชา" } else { "Course name" }}
                        prop:value=move || plan_name.get()
                        on:input=move |ev| set_plan_name.set(event_target_value(&ev))
                    />
                    <input
                        type="number"
                        min="0" max="12" step="1"
                        class="w-16 text-[13px] font-mono text-center px-2 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                        aria-label={move || if is_thai.get() { "หน่วยกิต" } else { "Credits" }}
                        prop:value=move || plan_credit.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                set_plan_credit.set(v);
                            }
                        }
                    />
                    <select
                        class="text-[13px] font-mono px-2 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                        aria-label={move || if is_thai.get() { "เกรดคาดการณ์" } else { "Expected grade" }}
                        on:change=move |ev| set_plan_grade.set(event_target_value(&ev))
                    >
                        <option value="A">"A"</option>
                        <option value="B+">"B+"</option>
                        <option value="B">"B"</option>
                        <option value="C+">"C+"</option>
                        <option value="C">"C"</option>
                        <option value="D+">"D+"</option>
                        <option value="D">"D"</option>
                        <option value="IP">"IP"</option>
                    </select>
                    <button
                        class="inline-flex items-center gap-1.5 text-xs font-semibold px-3 py-2 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-white dark:bg-zinc-50 dark:hover:bg-zinc-200 dark:text-zinc-900 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                        on:click=add_planned
                    >
                        <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15"/></svg>
                        <span>{move || if is_thai.get() { "เพิ่ม" } else { "Add" }}</span>
                    </button>
                </div>

                // Planned list
                {move || {
                    let planned = planned_courses.get();
                    (!planned.is_empty()).then(|| view! {
                        <div class="rounded-lg border border-zinc-200/80 dark:border-zinc-800 divide-y divide-zinc-100 dark:divide-zinc-800">
                            {planned.iter().enumerate().map(|(i, p)| {
                                // Prerequisites met by the transcript or by other planned courses.
                                let mut others = parsed_courses.get();
                                others.extend(planned.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, c)| c.clone()));
                                let unmet = unmet_prerequisites(&p.code, &others, curriculum(&curriculum_id.get()));
                                let code = p.code.clone();
                                let name = p.name.clone();
                                let credit = p.parsed_credit;
                                let grade = p.grade.clone();
                                view! {
                                    <div class="flex items-center justify-between px-4 py-2.5">
                                        <div class="flex items-center gap-3 min-w-0 flex-1">
                                            <span class="font-mono text-2xs font-semibold text-zinc-400 dark:text-zinc-500 w-16 shrink-0">{code}</span>
                                            <span class="text-[13px] text-zinc-700 dark:text-zinc-300 truncate">{name}</span>
                                            {(!unmet.is_empty()).then(|| {
                                                let list = unmet.join(", ");
                                                view! {
                                                    <span class="shrink-0 text-2xs font-semibold px-1.5 py-0.5 rounded bg-amber-50 text-amber-700 border border-amber-200/60 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-900/50" role="note">
                                                        {move || if is_thai.get() { format!("ต้องเรียน {} ก่อน", list) } else { format!("Requires {}", list) }}
                                                    </span>
                                                }
                                            })}
                                        </div>
                                        <div class="flex items-center gap-3 shrink-0 ml-3">
                                            <span class="text-2xs font-bold min-w-[1.75rem] h-5 px-1 flex items-center justify-center rounded border bg-purple-50 text-purple-700 border-purple-200/60 dark:bg-purple-950/40 dark:text-purple-300 dark:border-purple-900/50">{grade}</span>
                                            <span class="text-2xs text-zinc-400 dark:text-zinc-500 font-mono w-6 text-right">{format!("{}", credit as u32)}</span>
                                            <button
                                                class="text-zinc-400 hover:text-red-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500 rounded"
                                                aria-label={move || if is_thai.get() { "ลบวิชานี้จากแผน" } else { "Remove from plan" }}
                                                on:click=move |_| remove_planned(i)
                                            >
                                                <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    })
                }}

                // Clear button + hint
                {move || {
                    let planned = planned_courses.get();
                    (!planned.is_empty()).then(|| view! {
                        <div class="flex items-center justify-between pt-1">
                            <p class="text-2xs text-zinc-500 dark:text-zinc-400">{move || if is_thai.get() { format!("{} วิชาในแผน — ผลคาดการณ์ด้านล่างรวมแผนแล้ว", planned_courses.get().len()) } else { format!("{} planned courses — the projection below includes the plan", planned_courses.get().len()) }}</p>
                            <button
                                class="text-2xs font-semibold text-zinc-600 dark:text-zinc-400 hover:text-red-600 dark:hover:text-red-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500 rounded px-2 py-1"
                                on:click=clear_plan
                            >
                                {move || if is_thai.get() { "ล้างแผนทั้งหมด" } else { "Clear all" }}
                            </button>
                        </div>
                    })
                }}

                // Projection: transcript plus plan
                {move || plan_result.get().map(|r| {
                    let counted = r.counted_credits();
                    let required = r.required_credits();
                    let gpa = r.gpa.map_or("—".to_string(), |g| format!("{g:.2}"));
                    view! {
                        <p class="text-xs text-zinc-700 dark:text-zinc-300 tabular-nums">
                            {move || if is_thai.get() {
                                format!("ผลคาดการณ์ หน่วยกิตที่นับได้ {counted:.0} / {required:.0} · เกรดเฉลี่ย {gpa}")
                            } else {
                                format!("Projection: counted credits {counted:.0} / {required:.0} · GPA {gpa}")
                            }}
                        </p>
                    }
                })}
            </div>
        </div>
    }
}
