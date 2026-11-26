//! Plan tab: add hypothetical courses and see a projection; the real audit
//! result is left alone.

use audit_core::auditor::unmet_prerequisites;
use audit_core::data::curriculum;
use audit_core::models::ParsedCourse;
use audit_core::parser::normalize_course_code;
use leptos::prelude::*;

use crate::app::AppState;
use crate::components::status::{fmt_credits, status_line};
use crate::components::ui::{grade_class, CARD, SECTION_TITLE, TAG, TAG_OK, TAG_WARN};

#[component]
pub fn WhatIfSimulator() -> impl IntoView {
    let AppState {
        parsed_courses,
        audit_result,
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

    let field = "h-10 w-full min-w-0 rounded-[9px] border border-zinc-200 bg-white px-3 text-sm text-zinc-900 placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500";

    view! {
        <section class=format!("{CARD} overflow-hidden")>
            <div class="px-4 sm:px-[18px] pt-4 pb-1.5">
                <h2 class=SECTION_TITLE>{move || if is_thai.get() { "จำลองแผนการเรียน" } else { "Plan your next courses" }}</h2>
            </div>
            <p class="px-4 sm:px-[18px] pb-3 text-[13px] text-zinc-600">
                {move || if is_thai.get() {
                    "ใส่วิชาที่คิดจะลง แล้วดูว่าครบหลักสูตรหรือยัง และเกรดเฉลี่ยจะเป็นเท่าไร ผลจริงด้านบนไม่เปลี่ยน"
                } else {
                    "Add courses you plan to take and see whether you'd meet the curriculum and what your GPA would be. The real results above don't change."
                }}
            </p>
            <div class="grid grid-cols-1 md:grid-cols-[150px_minmax(0,1fr)_90px_90px_auto] gap-2.5 px-4 sm:px-[18px] pb-4">
                <input
                    type="text"
                    placeholder="344-495"
                    class=format!("{field} font-mono")
                    aria-label={move || if is_thai.get() { "รหัสวิชา" } else { "Course code" }}
                    prop:value=move || plan_code.get()
                    on:input=move |ev| set_plan_code.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder={move || if is_thai.get() { "ชื่อวิชา (ไม่บังคับ)" } else { "Course name (optional)" }}
                    class=field
                    aria-label={move || if is_thai.get() { "ชื่อวิชา" } else { "Course name" }}
                    prop:value=move || plan_name.get()
                    on:input=move |ev| set_plan_name.set(event_target_value(&ev))
                />
                <input
                    type="number"
                    min="0" max="12" step="1"
                    class=format!("{field} tabular-nums")
                    aria-label={move || if is_thai.get() { "หน่วยกิต" } else { "Credits" }}
                    prop:value=move || plan_credit.get().to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                            set_plan_credit.set(v);
                        }
                    }
                />
                <select
                    class=field
                    aria-label={move || if is_thai.get() { "เกรดที่คาด" } else { "Expected grade" }}
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
                    class="h-10 inline-flex items-center justify-center gap-1.5 rounded-[9px] bg-zinc-900 px-4 text-sm font-medium text-white hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 focus-visible:ring-offset-2"
                    on:click=add_planned
                >
                    <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15"/></svg>
                    {move || if is_thai.get() { "เพิ่มวิชา" } else { "Add course" }}
                </button>
            </div>

            // Planned list
            {move || {
                let planned = planned_courses.get();
                if planned.is_empty() {
                    return view! {
                        <p class="px-4 sm:px-[18px] py-5 text-center text-[13px] text-zinc-500 border-t border-zinc-200">
                            {move || if is_thai.get() { "ยังไม่มีวิชาในแผน" } else { "No courses planned yet" }}
                        </p>
                    }.into_any();
                }
                view! {
                    <div class="border-t border-zinc-200">
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
                                <div class="flex items-center gap-3 px-4 sm:px-[18px] py-2.5 border-b border-zinc-200 text-sm">
                                    <span class="font-mono text-[0.86em] text-zinc-500 w-16 shrink-0">{code}</span>
                                    <span class="min-w-0 flex-1 truncate text-zinc-800">{name}</span>
                                    {(!unmet.is_empty()).then(|| {
                                        let list = unmet.join(", ");
                                        view! {
                                            <span class=format!("{TAG} {TAG_WARN} shrink-0") role="note">
                                                {move || if is_thai.get() { format!("ต้องเรียน {list} ก่อน") } else { format!("Requires {list}") }}
                                            </span>
                                        }
                                    })}
                                    <span class="tabular-nums text-zinc-500 w-6 text-right">{fmt_credits(credit)}</span>
                                    <span class=grade_class(&grade)>{grade.clone()}</span>
                                    <button
                                        class="text-zinc-500 hover:text-red-600 rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                                        aria-label={move || if is_thai.get() { "ลบวิชานี้จากแผน" } else { "Remove from plan" }}
                                        on:click=move |_| remove_planned(i)
                                    >
                                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>
                                    </button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                        <div class="flex items-center justify-between px-4 sm:px-[18px] py-2">
                            <p class="text-xs text-zinc-500">
                                {move || if is_thai.get() { format!("{} วิชาในแผน", planned_courses.get().len()) } else { format!("{} planned courses", planned_courses.get().len()) }}
                            </p>
                            <button
                                class="text-xs font-medium text-zinc-500 hover:text-red-600 rounded px-1.5 py-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                                on:click=clear_plan
                            >
                                {move || if is_thai.get() { "ล้างแผนทั้งหมด" } else { "Clear all" }}
                            </button>
                        </div>
                    </div>
                }.into_any()
            }}

            // Projection: transcript plus plan (real result when nothing is planned)
            {move || {
                let r = plan_result.get().or_else(|| audit_result.get())?;
                let (ok, _, _) = status_line(&r, is_thai.get());
                let counted = r.counted_credits();
                let required = r.required_credits();
                let gpa = r.gpa.map_or("—".to_string(), |g| format!("{g:.2}"));
                let short = (required - counted).max(0.0);
                Some(view! {
                    <div class="grid grid-cols-1 min-[480px]:grid-cols-3 gap-px bg-zinc-200 border-t border-zinc-200">
                        <div class="bg-white px-4 sm:px-[18px] py-3.5">
                            <p class="text-[12.5px] text-zinc-500">{move || if is_thai.get() { "หน่วยกิตที่นับได้" } else { "Counted credits" }}</p>
                            <p class="text-xl font-semibold tabular-nums text-zinc-900">{format!("{} / {}", fmt_credits(counted), fmt_credits(required))}</p>
                        </div>
                        <div class="bg-white px-4 sm:px-[18px] py-3.5">
                            <p class="text-[12.5px] text-zinc-500">{move || if is_thai.get() { "เกรดเฉลี่ยคาดการณ์" } else { "Projected GPA" }}</p>
                            <p class="text-xl font-semibold tabular-nums text-zinc-900">{gpa}</p>
                        </div>
                        <div class="bg-white px-4 sm:px-[18px] py-3.5">
                            <p class="text-[12.5px] text-zinc-500">{move || if is_thai.get() { "สถานะ" } else { "Status" }}</p>
                            <p class="pt-1">
                                {if ok {
                                    view! { <span class=format!("{TAG} {TAG_OK}")>{move || if is_thai.get() { "ครบหลักสูตร" } else { "Requirements met" }}</span> }.into_any()
                                } else if short > 0.0 {
                                    view! { <span class=format!("{TAG} {TAG_WARN}")>{move || if is_thai.get() { format!("ขาดอีก {}", fmt_credits(short)) } else { format!("{} short", fmt_credits(short)) }}</span> }.into_any()
                                } else {
                                    view! { <span class=format!("{TAG} {TAG_WARN}")>{move || if is_thai.get() { "ยังไม่ครบ" } else { "Not complete" }}</span> }.into_any()
                                }}
                            </p>
                        </div>
                    </div>
                })
            }}
        </section>
    }
}
