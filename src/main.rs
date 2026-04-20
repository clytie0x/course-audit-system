//! Course Audit System - Rust + Leptos WebAssembly Application
//!
//! A privacy-first tool for Computer Science students at Prince of Songkla University (PSU).
//! Uploads transcripts locally as PDF, parses them, and audits progress against curriculum
//! requirements for General Education, Major, and Electives—all in the browser.
//!
//! **Key Features:**
//! - Client-side WASM application (no server calls)
//! - PDF parsing using Regex for course extraction
//! - Comprehensive curriculum auditing with sub-group support
//! - Greedy matching for repeatable courses
//! - Responsive Leptos UI with collapsible category cards

use leptos::*;
use leptos_meta::*;
use std::collections::HashSet;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DragEvent, Event, HtmlInputElement};

mod components;
mod data;
mod logic;
mod models;

use crate::components::category_card::CategoryCard;
use crate::data::{gen_ed::get_gen_ed_curriculum, major::get_major_curriculum};
use crate::logic::{
    auditor::{audit_gen_ed, audit_major, calculate_free_electives},
    parser::{extract_text_from_pdf, parse_transcript},
};
use crate::models::{
    compute_gpa, free_elective_dedupe_key, is_passing_grade, AuditResult, Category, Course,
    MissingCourse, ParseMode,
};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}

/// Runs the full audit pipeline (GenEd → Major → free electives → GPA) against
/// a course list and builds the UI-friendly `AuditResult`. Extracted so it can
/// be re-run when the What-If simulator's `planned_courses` changes.
fn build_audit(parsed: &[crate::models::ParsedCourse]) -> AuditResult {
    let gen_ed = get_gen_ed_curriculum();
    let major = get_major_curriculum();

    let (gen_ed_credits, gen_ed_missing, gen_ed_used) = audit_gen_ed(parsed, &gen_ed);
    let (major_credits, elective_credits, major_missing, major_used) = audit_major(parsed, &major);

    let mut all_used_courses = gen_ed_used.clone();
    all_used_courses.extend(major_used.clone());

    let (free_elective_credits, _) = calculate_free_electives(parsed, &all_used_courses);

    let mut all_missing: Vec<MissingCourse> = gen_ed_missing;
    all_missing.extend(major_missing);
    // Drop GenEd-wide misses when the 30-cr pool is met; Major/Core misses
    // are strict slots and stay regardless of totals.
    all_missing.retain(|m| match m.category.as_str() {
        "General Education" => gen_ed_credits < gen_ed.total_required_credits,
        _ => true,
    });

    let total_credits = gen_ed_credits + major_credits + elective_credits + free_elective_credits;

    let mut gen_ed_courses = Vec::new();
    let mut major_courses = Vec::new();
    let mut free_elective_courses = Vec::new();
    let mut seen_free_electives: HashSet<String> = HashSet::new();

    for (idx, p) in parsed.iter().enumerate() {
        let course = Course {
            code: p.code.clone(),
            name: p.name.clone(),
            credit: p.parsed_credit,
            grade: p.grade.clone(),
            semester: p.semester.clone(),
        };
        if gen_ed_used.contains(&idx) {
            gen_ed_courses.push(course);
        } else if major_used.contains(&idx) {
            major_courses.push(course);
        } else if is_passing_grade(&p.grade) {
            let dedupe_key = free_elective_dedupe_key(&p.code, &p.name);
            if seen_free_electives.insert(dedupe_key) {
                free_elective_courses.push(course);
            }
        }
    }

    let mut graded: Vec<Course> = Vec::new();
    graded.extend(gen_ed_courses.iter().cloned());
    graded.extend(major_courses.iter().cloned());
    graded.extend(free_elective_courses.iter().cloned());
    let gpa = compute_gpa(&graded);

    AuditResult {
        total_credits,
        categories: vec![
            Category {
                name: "General Education".to_string(),
                required_credits: gen_ed.total_required_credits,
                collected_credits: gen_ed_credits,
                courses: gen_ed_courses,
            },
            Category {
                name: "Major Courses".to_string(),
                required_credits: major.total_required_credits,
                collected_credits: major_credits + elective_credits,
                courses: major_courses,
            },
            Category {
                name: "Free Electives".to_string(),
                required_credits: 6.0,
                collected_credits: free_elective_credits,
                courses: free_elective_courses,
            },
        ],
        missing_subjects: all_missing,
        gpa,
    }
}

/// Main application component
///
/// Manages state for file upload, PDF preview, audit results, and loading state.
/// Implements drag-and-drop PDF upload with real-time PDF preview.
#[component]
fn App() -> impl IntoView {
    provide_meta_context();

    // State management for file upload and audit processing
    let (file_name, set_file_name) = create_signal(String::new());
    let (preview_url, set_preview_url) = create_signal(Option::<String>::None);
    let (audit_result, set_audit_result) = create_signal(Option::<AuditResult>::None);
    let (is_loading, set_is_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal(Option::<String>::None);
    // Stores the file from drag-and-drop (file input is not updated by drop events)
    let (dropped_file, set_dropped_file) = create_signal(Option::<web_sys::File>::None);
    // Which PDF format was detected during parsing (drives the mode banner).
    let (parse_mode, set_parse_mode) = create_signal(ParseMode::Grade);
    // Raw parsed courses from the PDF (kept around so the What-If simulator
    // can re-run audits without re-parsing).
    let (parsed_courses, set_parsed_courses) =
        create_signal(Vec::<crate::models::ParsedCourse>::new());
    // User-added hypothetical courses appended to the audit input.
    let (planned_courses, set_planned_courses) =
        create_signal(Vec::<crate::models::ParsedCourse>::new());
    // Form fields for the simulator
    let (plan_code, set_plan_code) = create_signal(String::new());
    let (plan_name, set_plan_name) = create_signal(String::new());
    let (plan_credit, set_plan_credit) = create_signal(3.0_f32);
    let (plan_grade, set_plan_grade) = create_signal("A".to_string());

    // Persistence — opt-in localStorage of the last audit result (key "audit_v1").
    // Remains off by default to stay privacy-first.
    let (save_enabled, set_save_enabled) = create_signal(false);

    // Try to rehydrate both the toggle and a prior result on mount.
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        if let Ok(Some(flag)) = storage.get_item("audit_save_enabled") {
            if flag == "1" {
                set_save_enabled.set(true);
                if let Ok(Some(raw)) = storage.get_item("audit_v1") {
                    if let Ok(saved) = serde_json::from_str::<AuditResult>(&raw) {
                        set_audit_result.set(Some(saved));
                    }
                }
                if let Ok(Some(mode_raw)) = storage.get_item("audit_mode_v1") {
                    if mode_raw == "registration" {
                        set_parse_mode.set(ParseMode::Registration);
                    }
                }
            }
        }
    }

    // Format the current result's missing list as a newline-delimited block
    // and copy to clipboard (Navigator.clipboard.writeText).
    let copy_missing = move |_| {
        let Some(result) = audit_result.get_untracked() else {
            return;
        };
        if result.missing_subjects.is_empty() {
            return;
        }
        let mut out = String::new();
        for m in &result.missing_subjects {
            out.push_str(&format!("[{}] {}\n", m.category, m.description));
        }
        if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
            let _ = clipboard.write_text(&out);
        }
    };

    // Download the AuditResult as a JSON file.
    let export_json = move |_| {
        let Some(result) = audit_result.get_untracked() else {
            return;
        };
        let Ok(json) = serde_json::to_string_pretty(&result) else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        // Build a Blob containing the JSON text and stream it via <a download>.
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(&json));
        let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) else {
            return;
        };
        let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
            return;
        };
        if let Ok(anchor) = document.create_element("a") {
            let _ = anchor.set_attribute("href", &url);
            let _ = anchor.set_attribute("download", "course-audit.json");
            if let Some(html_anchor) = anchor.dyn_ref::<web_sys::HtmlElement>() {
                html_anchor.click();
            }
        }
        let _ = web_sys::Url::revoke_object_url(&url);
    };

    // Trigger the browser's print dialog; style.css provides a print stylesheet.
    let print_report = move |_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    };

    // Re-run the audit whenever the parsed transcript or the simulator's
    // planned courses change. Skip when nothing has been parsed yet so the
    // empty-state keeps rendering.
    create_effect(move |_| {
        let parsed = parsed_courses.get();
        let planned = planned_courses.get();
        if parsed.is_empty() && planned.is_empty() {
            return;
        }
        let mut combined: Vec<crate::models::ParsedCourse> = parsed;
        combined.extend(planned);
        let result = build_audit(&combined);
        if save_enabled.get_untracked() {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                if let Ok(json) = serde_json::to_string(&result) {
                    let _ = storage.set_item("audit_v1", &json);
                }
            }
        }
        set_audit_result.set(Some(result));
    });

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
        let planned = crate::models::ParsedCourse {
            code: crate::logic::parser::normalize_course_code(&code),
            name: if name.is_empty() { code.clone() } else { name },
            grade,
            parsed_credit: credit,
            semester: Some(String::from("PLAN")),
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

    let toggle_save_enabled = move |_| {
        let new_val = !save_enabled.get_untracked();
        set_save_enabled.set(new_val);
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("audit_save_enabled", if new_val { "1" } else { "0" });
            if !new_val {
                let _ = storage.remove_item("audit_v1");
                let _ = storage.remove_item("audit_mode_v1");
            } else if let Some(result) = audit_result.get_untracked() {
                if let Ok(json) = serde_json::to_string(&result) {
                    let _ = storage.set_item("audit_v1", &json);
                }
                let mode = match parse_mode.get_untracked() {
                    ParseMode::Grade => "grade",
                    ParseMode::Registration => "registration",
                };
                let _ = storage.set_item("audit_mode_v1", mode);
            }
        }
    };

    // Language toggle: true = Thai (default), false = English
    let (is_thai, set_is_thai) = create_signal(true);
    provide_context(is_thai);

    // Dark-mode state. Initial value is sourced from the `dark` class applied
    // by the pre-paint script in index.html (honoring localStorage + system).
    let initial_dark = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .map(|el| el.class_list().contains("dark"))
        .unwrap_or(false);
    let (is_dark, set_is_dark) = create_signal(initial_dark);

    let toggle_dark = move |_| {
        set_is_dark.update(|v| *v = !*v);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.document_element() {
                let list = el.class_list();
                let want_dark = is_dark.get_untracked();
                if want_dark {
                    let _ = list.add_1("dark");
                } else {
                    let _ = list.remove_1("dark");
                }
            }
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                let _ = storage.set_item(
                    "theme",
                    if is_dark.get_untracked() {
                        "dark"
                    } else {
                        "light"
                    },
                );
            }
        }
    };

    // Handle file selection from input field
    let on_file_change = move |ev: Event| {
        let input = ev
            .target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
        if let Some(input) = input {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    set_file_name.set(file.name());
                    set_dropped_file.set(None); // file input takes precedence; clear any prior drop

                    // Revoke the previous blob URL to avoid memory leak, then create a new one
                    if let Some(old_url) = preview_url.get() {
                        let _ = web_sys::Url::revoke_object_url(&old_url);
                    }
                    // Create blob URL for PDF preview display
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) {
                        set_preview_url.set(Some(url));
                    }
                }
            }
        }
    };

    // Handle drag over
    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
    };

    // Handle drop
    let on_drop = move |ev: DragEvent| {
        ev.prevent_default();

        if let Some(data_transfer) = ev.data_transfer() {
            if let Some(files) = data_transfer.files() {
                if let Some(file) = files.get(0) {
                    if !file.name().to_lowercase().ends_with(".pdf") {
                        set_error_msg.set(Some(if is_thai.get_untracked() {
                            "กรุณาอัปโหลดไฟล์ PDF เท่านั้น".to_string()
                        } else {
                            "Please upload a PDF file.".to_string()
                        }));
                        return;
                    }
                    set_file_name.set(file.name());
                    set_dropped_file.set(Some(file.clone()));

                    // Revoke the previous blob URL to avoid memory leak, then create a new one
                    if let Some(old_url) = preview_url.get() {
                        let _ = web_sys::Url::revoke_object_url(&old_url);
                    }
                    // Create blob URL for PDF preview
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) {
                        set_preview_url.set(Some(url));
                    }
                }
            }
        }
    };

    // Handle start analysis
    let on_start_analysis = move |_| {
        if file_name.get().is_empty() {
            return;
        }

        set_is_loading.set(true);
        set_audit_result.set(None);
        set_error_msg.set(None);

        // Prefer file from drag-and-drop signal; fall back to the file-input element.
        // (drop events do not update the <input type="file"> files list)
        let file_opt = dropped_file.get().or_else(|| {
            web_sys::window()
                .ok_or(())
                .and_then(|w| w.document().ok_or(()))
                .and_then(|d| {
                    d.get_element_by_id("file-input")
                        .ok_or(())
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok().ok_or(()))
                })
                .ok()
                .and_then(|input| input.files())
                .and_then(|files| files.get(0))
        });

        if let Some(file) = file_opt {
            spawn_local(async move {
                use wasm_bindgen_futures::JsFuture;

                // gloo-file wraps the FileReader lifecycle cleanly: callbacks
                // are dropped when the future resolves, so no leaked closures
                // accumulate across repeated analyses.
                let gloo_file = gloo_file::File::from(file);
                let bytes = match gloo_file::futures::read_as_bytes(&gloo_file).await {
                    Ok(b) => b,
                    Err(_) => {
                        set_is_loading.set(false);
                        set_error_msg.set(Some(if is_thai.get_untracked() {
                            "ไม่สามารถอ่านไฟล์ PDF ได้ กรุณาลองใหม่อีกครั้ง".to_string()
                        } else {
                            "Failed to read the PDF file. Please try again.".to_string()
                        }));
                        return;
                    }
                };

                let uint8_array = js_sys::Uint8Array::from(bytes.as_slice());

                {
                    let promise = extract_text_from_pdf(uint8_array);
                    match JsFuture::from(promise).await {
                        Ok(text_value) => {
                            if let Some(text) = text_value.as_string() {
                                let (courses, mode) = parse_transcript(&text);
                                set_parse_mode.set(mode);
                                // Clear any prior planned courses — a fresh PDF should start clean.
                                set_planned_courses.set(Vec::new());
                                // Feed the audit effect. It'll build the AuditResult and persist
                                // to localStorage if save_enabled is on.
                                set_parsed_courses.set(courses);
                                if save_enabled.get_untracked() {
                                    if let Some(storage) = web_sys::window()
                                        .and_then(|w| w.local_storage().ok().flatten())
                                    {
                                        let mode_str = match mode {
                                            ParseMode::Grade => "grade",
                                            ParseMode::Registration => "registration",
                                        };
                                        let _ = storage.set_item("audit_mode_v1", mode_str);
                                    }
                                }
                                set_is_loading.set(false);
                            } else {
                                set_is_loading.set(false);
                                set_error_msg.set(Some(if is_thai.get_untracked() {
                                                "ไม่สามารถดึงข้อความจาก PDF กรุณาตรวจสอบว่าเป็นใบแสดงผลการเรียนที่ถูกต้อง".to_string()
                                            } else {
                                                "Could not extract text from the PDF. Make sure it's a valid transcript.".to_string()
                                            }));
                            }
                        }
                        Err(_e) => {
                            set_is_loading.set(false);
                            set_error_msg.set(Some(if is_thai.get_untracked() {
                                "การดึงข้อมูล PDF ล้มเหลว ไฟล์อาจเสียหายหรือถูกเข้ารหัส".to_string()
                            } else {
                                "PDF extraction failed. The file may be corrupted or encrypted."
                                    .to_string()
                            }));
                        }
                    }
                }
            });
        }
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/course-audit-system.css"/>
        <Title text="Course Audit — PSU CS"/>

        <div class="min-h-screen font-sans text-zinc-900 dark:text-zinc-100 flex flex-col selection:bg-brand-100 dark:selection:bg-brand-900/60">

            // ── Navbar ──────────────────────────────────────────────────
            <header class="sticky top-0 z-50 border-b border-zinc-200/60 dark:border-zinc-800/60 bg-white/80 dark:bg-zinc-950/80 backdrop-blur-xl backdrop-saturate-150">
                <div class="max-w-[1440px] mx-auto px-4 sm:px-6 lg:px-8 h-14 flex items-center justify-between">
                    <div class="flex items-center gap-3">
                        <div class="w-7 h-7 rounded-lg bg-brand-600 flex items-center justify-center">
                            <svg class="w-4 h-4 text-white" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.2" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M4.26 10.147a60.436 60.436 0 00-.491 6.347A48.627 48.627 0 0112 20.904a48.627 48.627 0 018.232-4.41 60.46 60.46 0 00-.491-6.347m-15.482 0a50.57 50.57 0 00-2.658-.813A59.905 59.905 0 0112 3.493a59.902 59.902 0 0110.399 5.84c-.896.248-1.783.52-2.658.814m-15.482 0A50.697 50.697 0 0112 13.489a50.702 50.702 0 017.74-3.342"/>
                            </svg>
                        </div>
                        <span class="text-[15px] font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">"Course Audit"</span>
                        <span class="hidden sm:inline text-xs font-medium text-zinc-400 dark:text-zinc-500 bg-zinc-100 dark:bg-zinc-800 px-2 py-0.5 rounded-md">"CS · PSU"</span>
                    </div>
                    <div class="flex items-center gap-3">
                        <button
                            class="text-xs font-semibold min-w-[2.5rem] min-h-[2.25rem] px-3 py-1.5 rounded-lg bg-zinc-100 hover:bg-zinc-200 text-zinc-600 dark:bg-zinc-800 dark:hover:bg-zinc-700 dark:text-zinc-300 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                            aria-label={move || if is_thai.get() { "Switch to English" } else { "เปลี่ยนเป็นภาษาไทย" }}
                            on:click=move |_| set_is_thai.update(|v| *v = !*v)
                        >
                            {move || if is_thai.get() { "EN" } else { "ไทย" }}
                        </button>
                        <button
                            class="w-9 h-9 rounded-lg bg-zinc-100 hover:bg-zinc-200 text-zinc-600 dark:bg-zinc-800 dark:hover:bg-zinc-700 dark:text-zinc-300 transition-colors flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                            aria-label={move || if is_dark.get() {
                                if is_thai.get() { "เปลี่ยนเป็นโหมดสว่าง" } else { "Switch to light mode" }
                            } else {
                                if is_thai.get() { "เปลี่ยนเป็นโหมดมืด" } else { "Switch to dark mode" }
                            }}
                            on:click=toggle_dark
                        >
                            {move || if is_dark.get() {
                                view! {
                                    <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z"/>
                                    </svg>
                                }.into_view()
                            } else {
                                view! {
                                    <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0118 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 003 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 009.002-5.998z"/>
                                    </svg>
                                }.into_view()
                            }}
                        </button>
                        <div class="flex items-center gap-2 text-xs text-zinc-400 dark:text-zinc-500">
                            <div class="w-1.5 h-1.5 rounded-full bg-emerald-400 pulse-dot"></div>
                            <span class="hidden sm:inline font-medium">{move || if is_thai.get() { "100% ประมวลผลบนเครื่อง" } else { "100% client-side" }}</span>
                        </div>
                    </div>
                </div>
            </header>

            // ── Main Content ────────────────────────────────────────────
            <div class="flex-1 flex flex-col lg:flex-row max-w-[1440px] w-full mx-auto p-4 sm:p-6 gap-5 animate-fade-in">

                // ── Left Sidebar ────────────────────────────────────────
                <aside class="w-full lg:w-[360px] shrink-0 flex flex-col gap-4">

                    // Upload Card
                    <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft p-5 flex flex-col gap-4">
                        <div>
                            <h2 class="text-base font-semibold text-zinc-900 dark:text-zinc-100 tracking-tight">{move || if is_thai.get() { "อัปโหลดใบแสดงผลการเรียน" } else { "Upload Transcript" }}</h2>
                            <p class="text-[13px] text-zinc-500 dark:text-zinc-400 mt-0.5 leading-relaxed">{move || if is_thai.get() { "PDF ของคุณถูกประมวลผลในเบราว์เซอร์ทั้งหมด ข้อมูลไม่ออกจากอุปกรณ์ของคุณ" } else { "Your PDF is processed entirely in the browser. Nothing leaves your device." }}</p>
                        </div>

                        // Drop zone
                        <div
                            class="group relative border border-dashed border-zinc-300 dark:border-zinc-700 rounded-xl p-5 text-center bg-zinc-50/50 dark:bg-zinc-800/30 hover:border-brand-400 hover:bg-brand-50/30 dark:hover:border-brand-500 dark:hover:bg-brand-950/20 transition-all duration-200 cursor-pointer focus-within:border-brand-500 focus-within:ring-2 focus-within:ring-brand-500/30"
                            on:dragover=on_drag_over
                            on:drop=on_drop
                        >
                            <input
                                type="file"
                                accept="application/pdf"
                                class="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
                                id="file-input"
                                aria-label={move || if is_thai.get() { "อัปโหลดใบแสดงผลการเรียน PDF" } else { "Upload transcript PDF" }}
                                on:change=on_file_change
                            />
                            <div class="flex flex-col items-center gap-2.5 pointer-events-none">
                                <div class="w-10 h-10 rounded-xl bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 text-zinc-400 dark:text-zinc-500 flex items-center justify-center group-hover:text-brand-500 group-hover:border-brand-200 group-hover:bg-brand-50 dark:group-hover:bg-brand-950/40 transition-colors">
                                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"/>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-sm font-medium text-zinc-700 dark:text-zinc-300 group-hover:text-brand-600 dark:group-hover:text-brand-400 transition-colors">{move || if is_thai.get() { "วางไฟล์ PDF ที่นี่ หรือคลิกเพื่อเลือกไฟล์" } else { "Drop PDF here or click to browse" }}</p>
                                    <p class="text-2xs text-zinc-400 dark:text-zinc-500 mt-0.5">{move || if is_thai.get() { "รับเฉพาะไฟล์ .pdf เท่านั้น" } else { "Accepts .pdf files only" }}</p>
                                </div>
                            </div>
                        </div>

                        // Selected file
                        {move || (!file_name.get().is_empty()).then(|| view! {
                            <div class="flex items-center gap-2.5 px-3 py-2.5 rounded-lg bg-emerald-50 border border-emerald-200/60 animate-scale-in">
                                <svg class="w-4 h-4 text-emerald-500 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                </svg>
                                <p class="text-[13px] text-emerald-800 font-medium truncate">{file_name.get()}</p>
                            </div>
                        })}

                        // Analyze button
                        <button
                            class="w-full flex items-center justify-center gap-2 bg-zinc-900 hover:bg-zinc-800 dark:bg-brand-600 dark:hover:bg-brand-500 text-white text-sm font-medium py-2.5 px-4 rounded-xl transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed active:scale-[0.98] shadow-soft hover:shadow-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                            disabled={move || file_name.get().is_empty() || is_loading.get()}
                            on:click=on_start_analysis
                        >
                            {move || if is_loading.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4 text-white/70" aria-hidden="true" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/></svg>
                                    <span>{move || if is_thai.get() { "กำลังวิเคราะห์..." } else { "Analyzing..." }}</span>
                                }.into_view()
                            } else {
                                view! {
                                    <span>{move || if is_thai.get() { "วิเคราะห์ใบแสดงผลการเรียน" } else { "Analyze Transcript" }}</span>
                                    <svg class="w-4 h-4 opacity-50" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3"/></svg>
                                }.into_view()
                            }}
                        </button>
                    </div>

                    // How it works card (only when no file selected)
                    {move || preview_url.get().is_none().then(|| view! {
                        <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft p-5 animate-fade-in-up">
                            <h3 class="text-xs font-semibold text-zinc-500 dark:text-zinc-400 uppercase tracking-widest mb-3">{move || if is_thai.get() { "วิธีการใช้งาน" } else { "How it works" }}</h3>
                            <div class="space-y-3">
                                <div class="flex items-start gap-3">
                                    <div class="mt-0.5 w-5 h-5 shrink-0 rounded-full bg-zinc-900 dark:bg-brand-600 text-white flex items-center justify-center text-2xs font-bold">"1"</div>
                                    <p class="text-[13px] text-zinc-600 dark:text-zinc-400 leading-relaxed">{move || if is_thai.get() { "ดาวน์โหลดใบแสดงผลการเรียน PDF จากระบบ SIS" } else { "Download your unofficial transcript PDF from SIS." }}</p>
                                </div>
                                <div class="flex items-start gap-3">
                                    <div class="mt-0.5 w-5 h-5 shrink-0 rounded-full bg-zinc-900 dark:bg-brand-600 text-white flex items-center justify-center text-2xs font-bold">"2"</div>
                                    <p class="text-[13px] text-zinc-600 dark:text-zinc-400 leading-relaxed">{move || if is_thai.get() { "อัปโหลดไฟล์ PDF ผ่านพื้นที่วางไฟล์ด้านบน" } else { "Upload the PDF using the drop zone above." }}</p>
                                </div>
                                <div class="flex items-start gap-3">
                                    <div class="mt-0.5 w-5 h-5 shrink-0 rounded-full bg-zinc-900 dark:bg-brand-600 text-white flex items-center justify-center text-2xs font-bold">"3"</div>
                                    <p class="text-[13px] text-zinc-600 dark:text-zinc-400 leading-relaxed">{move || if is_thai.get() { "คลิก วิเคราะห์ — ทุกอย่างประมวลผลบนเครื่องของคุณเพื่อความเป็นส่วนตัวสูงสุด" } else { "Click Analyze — everything is processed locally for maximum privacy." }}</p>
                                </div>
                            </div>
                        </div>
                    })}

                    // Save session toggle (opt-in localStorage)
                    <label class="flex items-center gap-3 px-4 py-3 rounded-xl bg-white dark:bg-zinc-900 border border-zinc-200/80 dark:border-zinc-800 cursor-pointer hover:border-brand-300 dark:hover:border-brand-700 transition-colors">
                        <input
                            type="checkbox"
                            class="w-4 h-4 rounded border-zinc-300 dark:border-zinc-600 text-brand-600 focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
                            prop:checked=move || save_enabled.get()
                            on:change=toggle_save_enabled
                        />
                        <div class="flex-1 min-w-0">
                            <p class="text-[13px] font-medium text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "บันทึกผลในเบราว์เซอร์" } else { "Save results in browser" }}</p>
                            <p class="text-2xs text-zinc-500 dark:text-zinc-400 mt-0.5 leading-relaxed">{move || if is_thai.get() { "ผลลัพธ์จะอยู่ใน localStorage (ไม่ส่งออกนอกเครื่อง)" } else { "Kept in localStorage only — never leaves your device." }}</p>
                        </div>
                    </label>

                    // Privacy badge
                    <div class="hidden lg:flex items-center gap-2 px-4 py-3 rounded-xl bg-zinc-50 dark:bg-zinc-900/60 border border-zinc-100 dark:border-zinc-800">
                        <svg class="w-4 h-4 text-zinc-400 dark:text-zinc-500 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z"/>
                        </svg>
                        <p class="text-2xs text-zinc-500 dark:text-zinc-400 font-medium">{move || if is_thai.get() { "ใบแสดงผลการเรียนของคุณไม่ออกจากอุปกรณ์นี้ ทุกอย่างประมวลผลใน WebAssembly" } else { "Your transcript never leaves this device. All processing runs in WebAssembly." }}</p>
                    </div>
                </aside>

                // ── Right Panel — Results ────────────────────────────────
                <main class="flex-1 min-w-0" aria-live="polite" aria-atomic="false">
                    {move || {
                        if is_loading.get() {
                            // Loading state
                            view! {
                                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft h-full min-h-[500px] flex flex-col items-center justify-center gap-4" role="status" aria-live="polite">
                                    <div class="relative">
                                        <div class="w-12 h-12 rounded-full border-2 border-zinc-200 dark:border-zinc-700"></div>
                                        <div class="absolute inset-0 w-12 h-12 rounded-full border-2 border-brand-500 border-t-transparent animate-spin"></div>
                                    </div>
                                    <div class="text-center">
                                        <p class="text-sm font-medium text-zinc-700 dark:text-zinc-300">{move || if is_thai.get() { "กำลังวิเคราะห์ใบแสดงผลการเรียน..." } else { "Analyzing transcript..." }}</p>
                                        <p class="text-xs text-zinc-400 dark:text-zinc-500 mt-1">{move || if is_thai.get() { "กำลังดึงข้อมูลวิชาและตรวจสอบข้อกำหนด" } else { "Parsing courses and validating requirements" }}</p>
                                    </div>
                                </div>
                            }.into_view()
                        } else if let Some(err) = error_msg.get() {
                            // Error state
                            view! {
                                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft h-full min-h-[500px] flex flex-col items-center justify-center gap-4 px-8 text-center" role="alert">
                                    <div class="w-12 h-12 rounded-full bg-red-50 dark:bg-red-950/40 flex items-center justify-center">
                                        <svg class="w-6 h-6 text-red-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"/>
                                        </svg>
                                    </div>
                                    <div>
                                        <p class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "การวิเคราะห์ล้มเหลว" } else { "Analysis Failed" }}</p>
                                        <p class="text-[13px] text-zinc-500 dark:text-zinc-400 mt-1 max-w-sm leading-relaxed">{err}</p>
                                    </div>
                                </div>
                            }.into_view()
                        } else if let Some(result) = audit_result.get() {
                            // Results view
                            view! {
                                <div class="space-y-5 animate-fade-in">

                                    // ── Actions toolbar (export / print) ─────
                                    <div class="flex items-center justify-end gap-2 print:hidden">
                                        <button
                                            class="inline-flex items-center gap-1.5 text-xs font-medium px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-950"
                                            on:click=export_json
                                        >
                                            <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"/></svg>
                                            <span>{move || if is_thai.get() { "ดาวน์โหลด JSON" } else { "Export JSON" }}</span>
                                        </button>
                                        <button
                                            class="inline-flex items-center gap-1.5 text-xs font-medium px-3 py-1.5 rounded-lg border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300 hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-950"
                                            on:click=print_report
                                        >
                                            <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5zm-3 0h.008v.008H15V10.5z"/></svg>
                                            <span>{move || if is_thai.get() { "พิมพ์ / บันทึก PDF" } else { "Print / Save PDF" }}</span>
                                        </button>
                                    </div>

                                    // ── Registration-mode banner ─────────────
                                    {move || (parse_mode.get() == ParseMode::Registration).then(|| view! {
                                        <div class="rounded-2xl border border-amber-200/60 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/30 px-5 py-3 flex items-start gap-3" role="status">
                                            <svg class="w-5 h-5 text-amber-500 shrink-0 mt-0.5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m0-10.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285zM12 15.75h.007v.008H12v-.008z"/></svg>
                                            <div class="text-[13px] text-amber-900 dark:text-amber-200 leading-relaxed">
                                                <p class="font-semibold">{move || if is_thai.get() { "กำลังดูไฟล์ผลการลงทะเบียน" } else { "Viewing registration file" }}</p>
                                                <p class="text-amber-800/90 dark:text-amber-300/90 mt-0.5">{move || if is_thai.get() { "วิชาที่กำลังเรียนแสดงเป็นแท็กสีเหลือง และยังไม่นับหน่วยกิตจนกว่าจะได้เกรด" } else { "In-progress courses appear with a yellow tag and don't count toward credits until graded." }}</p>
                                            </div>
                                        </div>
                                    })}

                                    // ── Hero: Total Credits ─────────────────
                                    <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft p-6 sm:p-8 relative overflow-hidden">
                                        <div class="absolute -right-16 -top-16 w-48 h-48 bg-brand-100/40 dark:bg-brand-900/20 rounded-full blur-3xl pointer-events-none"></div>
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

                                    // ── Category Progress Cards ─────────────
                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 stagger-in">
                                        {result.categories.iter().map(|cat| {
                                            let pct = ((cat.collected_credits / cat.required_credits) * 100.0).min(100.0);
                                            let complete = pct >= 100.0;
                                            let cat_name_str = cat.name.clone();
                                            let cat_display = {
                                                let cat_name_str = cat_name_str.clone();
                                                move || {
                                                    let is_thai = is_thai.get();
                                                    match cat_name_str.as_str() {
                                                        "General Education" if is_thai => "หมวดวิชาศึกษาทั่วไป".to_string(),
                                                        "Major Courses" if is_thai => "หมวดวิชาเฉพาะ".to_string(),
                                                        "Free Electives" if is_thai => "หมวดวิชาเลือกเสรี".to_string(),
                                                        _ => cat_name_str.clone(),
                                                    }
                                                }
                                            };
                                            let collected = cat.collected_credits;
                                            let required = cat.required_credits;
                                            let pct_display = pct as u32;

                                            // SVG donut params
                                            let circumference = 100.0;
                                            let dash = (pct / 100.0) * circumference;

                                            let color_class = if complete { "text-emerald-500" } else { "text-brand-500" };
                                            let bg_class = if complete { "bg-emerald-50 border-emerald-100 dark:bg-emerald-950/30 dark:border-emerald-900/40" } else { "bg-white border-zinc-200/80 dark:bg-zinc-900 dark:border-zinc-800" };
                                            let badge_class = if complete { "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300" } else { "bg-brand-50 text-brand-600 dark:bg-brand-950/40 dark:text-brand-300" };
                                            let badge_text = move || if is_thai.get() {
                                                if complete { "เสร็จสมบูรณ์" } else { "กำลังดำเนินการ" }
                                            } else {
                                                if complete { "Complete" } else { "In Progress" }
                                            };

                                            view! {
                                                <div class={format!("rounded-2xl border shadow-soft p-5 flex flex-col gap-4 transition-shadow hover:shadow-medium {}", bg_class)}>
                                                    <div class="flex items-start justify-between">
                                                        <p class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{cat_display}</p>
                                                        <span class={format!("text-2xs font-semibold px-2 py-0.5 rounded-full {}", badge_class)}>{badge_text}</span>
                                                    </div>
                                                    <div class="flex items-center gap-4">
                                                        // Donut chart
                                                        <div class="relative w-14 h-14 shrink-0">
                                                            <svg class="w-14 h-14 -rotate-90" viewBox="0 0 36 36" role="img" aria-label={format!("{}% complete", pct_display)}>
                                                                <circle cx="18" cy="18" r="15.9155" fill="none" class="stroke-zinc-200 dark:stroke-zinc-700" stroke-width="3"/>
                                                                <circle cx="18" cy="18" r="15.9155" fill="none"
                                                                    class={format!("{} donut-animated", color_class)}
                                                                    stroke="currentColor" stroke-width="3" stroke-linecap="round"
                                                                    stroke-dasharray={format!("{} {}", dash, circumference - dash)}/>
                                                            </svg>
                                                            <div class="absolute inset-0 flex items-center justify-center">
                                                                <span class="text-xs font-bold text-zinc-700 dark:text-zinc-200" aria-hidden="true">{format!("{}%", pct_display)}</span>
                                                            </div>
                                                        </div>
                                                        // Credits
                                                        <div>
                                                            <div class="flex items-baseline gap-1">
                                                                <span class="text-2xl font-bold text-zinc-900 dark:text-zinc-100 tabular-nums">{collected as u32}</span>
                                                                <span class="text-sm text-zinc-400 dark:text-zinc-500 font-medium">{format!("/ {}", required as u32)}</span>
                                                            </div>
                                                            <p class="text-2xs text-zinc-400 dark:text-zinc-500 mt-0.5 font-medium">{move || if is_thai.get() { "หน่วยกิต" } else { "credits" }}</p>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>

                                    // ── Course Details Accordion ─────────────
                                    <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft overflow-hidden">
                                        <div class="px-5 py-4 border-b border-zinc-100 dark:border-zinc-800 flex items-center gap-2.5">
                                            <svg class="w-4 h-4 text-zinc-400 dark:text-zinc-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 010 3.75H5.625a1.875 1.875 0 010-3.75z"/></svg>
                                            <h3 class="text-sm font-semibold text-zinc-800 dark:text-zinc-200">{move || if is_thai.get() { "รายละเอียดวิชา" } else { "Course Details" }}</h3>
                                        </div>
                                        <div class="divide-y divide-zinc-100 dark:divide-zinc-800">
                                            {result.categories.iter().map(|category| {
                                                let category = category.clone();
                                                view! { <CategoryCard category={category} /> }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>

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
                                                    class="text-[13px] font-mono px-3 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
                                                    aria-label={move || if is_thai.get() { "รหัสวิชา" } else { "Course code" }}
                                                    prop:value=move || plan_code.get()
                                                    on:input=move |ev| set_plan_code.set(event_target_value(&ev))
                                                />
                                                <input
                                                    type="text"
                                                    placeholder={move || if is_thai.get() { "ชื่อวิชา (ไม่จำเป็น)" } else { "Course name (optional)" }}
                                                    class="text-[13px] px-3 py-2 rounded-lg bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
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
                                                    class="inline-flex items-center gap-1.5 text-xs font-semibold px-3 py-2 rounded-lg bg-brand-600 hover:bg-brand-500 text-white transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2 dark:focus-visible:ring-offset-zinc-900"
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
                                                            let code = p.code.clone();
                                                            let name = p.name.clone();
                                                            let credit = p.parsed_credit;
                                                            let grade = p.grade.clone();
                                                            view! {
                                                                <div class="flex items-center justify-between px-4 py-2.5">
                                                                    <div class="flex items-center gap-3 min-w-0 flex-1">
                                                                        <span class="font-mono text-2xs font-semibold text-zinc-400 dark:text-zinc-500 w-16 shrink-0">{code}</span>
                                                                        <span class="text-[13px] text-zinc-700 dark:text-zinc-300 truncate">{name}</span>
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
                                                        <p class="text-2xs text-zinc-500 dark:text-zinc-400">{move || if is_thai.get() { format!("{} วิชาในแผน — ผลลัพธ์ด้านบนรวมแผนแล้ว", planned_courses.get().len()) } else { format!("{} planned courses — results above include the plan", planned_courses.get().len()) }}</p>
                                                        <button
                                                            class="text-2xs font-semibold text-zinc-600 dark:text-zinc-400 hover:text-red-600 dark:hover:text-red-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500 rounded px-2 py-1"
                                                            on:click=clear_plan
                                                        >
                                                            {move || if is_thai.get() { "ล้างแผนทั้งหมด" } else { "Clear all" }}
                                                        </button>
                                                    </div>
                                                })
                                            }}
                                        </div>
                                    </div>

                                    // ── Semester-by-semester breakdown ──────
                                    {
                                        // Gather every course (across all categories) that has
                                        // a semester tag and group them by semester label.
                                        let mut by_sem: Vec<(String, Vec<Course>)> = Vec::new();
                                        for cat in &result.categories {
                                            for course in &cat.courses {
                                                if let Some(sem) = &course.semester {
                                                    if let Some(bucket) = by_sem.iter_mut().find(|(s, _)| s == sem) {
                                                        bucket.1.push(course.clone());
                                                    } else {
                                                        by_sem.push((sem.clone(), vec![course.clone()]));
                                                    }
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

                                    // ── Missing Requirements ─────────────────
                                    {(!result.missing_subjects.is_empty()).then(|| {
                                        let mut seen_cats: Vec<String> = Vec::new();
                                        for m in &result.missing_subjects {
                                            if !seen_cats.contains(&m.category) {
                                                seen_cats.push(m.category.clone());
                                            }
                                        }
                                        let missing_count = result.missing_subjects.len();
                                        view! {
                                            <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-red-200/60 dark:border-red-900/40 shadow-soft overflow-hidden">
                                                <div class="px-5 py-4 border-b border-red-100 dark:border-red-900/40 flex items-center gap-2.5 bg-red-50/50 dark:bg-red-950/20">
                                                    <svg class="w-4 h-4 text-red-500" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"/></svg>
                                                    <h3 class="text-sm font-semibold text-red-800 dark:text-red-300">{move || if is_thai.get() { "ข้อกำหนดที่ขาด" } else { "Missing Requirements" }}</h3>
                                                    <span class="ml-auto text-2xs font-semibold text-red-600 dark:text-red-300 bg-red-100 dark:bg-red-900/40 px-2 py-0.5 rounded-full">{move || if is_thai.get() { format!("{} รายการ", missing_count) } else { format!("{} items", missing_count) }}</span>
                                                    <button
                                                        class="print:hidden inline-flex items-center gap-1 text-2xs font-semibold text-red-700 dark:text-red-300 hover:text-red-900 dark:hover:text-red-100 border border-red-200 dark:border-red-900/50 hover:bg-red-100 dark:hover:bg-red-900/30 px-2 py-0.5 rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500"
                                                        aria-label={move || if is_thai.get() { "คัดลอกรายการที่ขาด" } else { "Copy missing items" }}
                                                        on:click=copy_missing
                                                    >
                                                        <svg class="w-3 h-3" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 01-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 011.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 00-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 01-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 00-3.375-3.375h-1.5a1.125 1.125 0 01-1.125-1.125v-1.5a3.375 3.375 0 00-3.375-3.375H9.75"/></svg>
                                                        <span>{move || if is_thai.get() { "คัดลอก" } else { "Copy" }}</span>
                                                    </button>
                                                </div>
                                                <div class="divide-y divide-red-100/60 dark:divide-red-900/30">
                                                    {seen_cats.iter().map(|cat| {
                                                        let cat_courses: Vec<_> = result.missing_subjects.iter()
                                                            .filter(|m| &m.category == cat)
                                                            .collect();
                                                        let display_items: Vec<String> = if cat == "General Education" {
                                                            let mut ge_groups: Vec<String> = Vec::new();
                                                            for m in &cat_courses {
                                                                let description = m.description.trim();
                                                                let group = if description.contains("missing") {
                                                                    description.to_string()
                                                                } else {
                                                                    description.split(':').next().unwrap_or(description).trim().to_string()
                                                                };
                                                                if !ge_groups.contains(&group) {
                                                                    ge_groups.push(group);
                                                                }
                                                            }
                                                            ge_groups
                                                        } else {
                                                            cat_courses.iter().map(|m| m.description.clone()).collect()
                                                        };
                                                        let cat_display_name = {
                                                            let name = cat.clone();
                                                            move || {
                                                                let is_thai = is_thai.get();
                                                                match name.as_str() {
                                                                    "General Education" if is_thai => "หมวดวิชาศึกษาทั่วไป".to_string(),
                                                                    "Major Courses" if is_thai => "หมวดวิชาเฉพาะ".to_string(),
                                                                    "Free Electives" if is_thai => "หมวดวิชาเลือกเสรี".to_string(),
                                                                    _ => name.clone(),
                                                                }
                                                            }
                                                        };
                                                        view! {
                                                            <div class="p-5">
                                                                <p class="text-xs font-semibold text-zinc-700 dark:text-zinc-300 uppercase tracking-wider mb-2.5">{cat_display_name}</p>
                                                                <div class="space-y-1.5">
                                                                    {display_items.iter().map(|item| {
                                                                        let desc = item.clone();
                                                                        view! {
                                                                            <div class="flex items-start gap-2.5 py-1.5">
                                                                                <div class="w-1.5 h-1.5 rounded-full bg-red-400 dark:bg-red-500 mt-1.5 shrink-0"></div>
                                                                                <p class="text-[13px] text-zinc-600 dark:text-zinc-300 leading-relaxed">{desc}</p>
                                                                            </div>
                                                                        }
                                                                    }).collect::<Vec<_>>()}
                                                                </div>
                                                            </div>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            </div>
                                        }
                                    })}
                                </div>
                            }.into_view()
                        } else {
                            // Empty state
                            view! {
                                <div class="bg-white dark:bg-zinc-900 rounded-2xl border border-zinc-200/80 dark:border-zinc-800 shadow-soft h-full min-h-[500px] flex flex-col items-center justify-center gap-5 px-8 text-center">
                                    <div class="w-16 h-16 rounded-2xl bg-zinc-50 dark:bg-zinc-800/50 border border-zinc-200 dark:border-zinc-700 flex items-center justify-center">
                                        <svg class="w-7 h-7 text-zinc-300 dark:text-zinc-600" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"/>
                                        </svg>
                                    </div>
                                    <div>
                                        <h3 class="text-base font-semibold text-zinc-800 dark:text-zinc-200 mb-1">{move || if is_thai.get() { "ยังไม่ได้อัปโหลดใบแสดงผลการเรียน" } else { "No transcript uploaded" }}</h3>
                                        <p class="text-sm text-zinc-400 dark:text-zinc-500 max-w-xs leading-relaxed">{move || if is_thai.get() { "อัปโหลดใบแสดงผลการเรียน PDF เพื่อดูความคืบหน้าในการสำเร็จการศึกษา" } else { "Upload your PDF transcript to see a breakdown of your progress toward graduation." }}</p>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </main>
            </div>
        </div>
    }
}
