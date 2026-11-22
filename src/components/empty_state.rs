//! Before a transcript is analysed: drop zone, curriculum choice, analyse
//! button, how-it-works steps and the save toggle. Loading and errors show in
//! place of the drop zone.

use crate::pdf::extract_text_from_pdf;
use audit_core::data::{curricula, curriculum_for_cohort};
use audit_core::models::ParseMode;
use audit_core::parser::{detect_cohort, parse_transcript};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::DragEvent;
use web_sys::Event;
use web_sys::HtmlInputElement;

use crate::app::AppState;

/// Turns "save results in this browser" on or off and syncs localStorage.
pub fn apply_save_choice(state: AppState, new_val: bool) {
    let AppState {
        audit_result,
        parse_mode,
        curriculum_id,
        parsed_courses,
        set_save_enabled,
        ..
    } = state;
    set_save_enabled.set(new_val);
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item("audit_save_enabled", if new_val { "1" } else { "0" });
        if !new_val {
            let _ = storage.remove_item("audit_v1");
            let _ = storage.remove_item("audit_mode_v1");
            let _ = storage.remove_item("audit_parsed_v1");
            let _ = storage.remove_item("audit_curriculum_v1");
            let _ = storage.remove_item("audit_cohort_v1");
        } else if let Some(result) = audit_result.get_untracked() {
            if let Ok(json) = serde_json::to_string(&parsed_courses.get_untracked()) {
                let _ = storage.set_item("audit_parsed_v1", &json);
            }
            let _ = storage.set_item("audit_curriculum_v1", &curriculum_id.get_untracked());
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
}

/// "Change file": drop the result, transcript and plan and show the empty state.
pub fn reset_to_empty(state: AppState) {
    if let Some(old_url) = state.preview_url.get_untracked() {
        let _ = web_sys::Url::revoke_object_url(&old_url);
    }
    state.set_preview_url.set(None);
    state.set_planned_courses.set(Vec::new());
    state.set_parsed_courses.set(Vec::new());
    state.set_audit_result.set(None);
    state.set_file_name.set(String::new());
    state.set_dropped_file.set(None);
    state.set_error_msg.set(None);
}

/// Checkbox row for the opt-in localStorage save.
#[component]
pub fn SaveToggle() -> impl IntoView {
    let state = expect_context::<AppState>();
    let AppState {
        save_enabled,
        is_thai,
        ..
    } = state;
    view! {
        <label class="inline-flex items-start gap-2.5 cursor-pointer text-left">
            <input
                type="checkbox"
                class="mt-1 w-4 h-4 rounded border-zinc-300 accent-zinc-900 focus-visible:ring-2 focus-visible:ring-accent-500"
                prop:checked=move || save_enabled.get()
                on:change=move |_| apply_save_choice(state, !save_enabled.get_untracked())
            />
            <span>
                <span class="block text-sm font-medium text-zinc-800">{move || if is_thai.get() { "บันทึกผลไว้ในเบราว์เซอร์นี้" } else { "Save results in this browser" }}</span>
                <span class="block text-xs text-zinc-500">{move || if is_thai.get() { "เก็บใน localStorage ของเครื่องนี้เท่านั้น" } else { "Kept in this device's localStorage only" }}</span>
            </span>
        </label>
    }
}

#[component]
pub fn EmptyState() -> impl IntoView {
    let AppState {
        file_name,
        set_file_name,
        preview_url,
        set_preview_url,
        set_audit_result,
        is_loading,
        set_is_loading,
        error_msg,
        set_error_msg,
        dropped_file,
        set_dropped_file,
        set_parse_mode,
        curriculum_id,
        set_curriculum_id,
        set_cohort,
        set_parsed_courses,
        set_planned_courses,
        save_enabled,
        is_thai,
        ..
    } = expect_context::<AppState>();
    // "Auto" picks the curriculum from the student ID; a hand-picked one wins.
    let (auto_curriculum, set_auto_curriculum) = signal(true);

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

        // Read now: this component is replaced by the results before the task ends.
        let auto = auto_curriculum.get_untracked();
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
                                // Pick the curriculum revision from the cohort in the student ID.
                                let detected = detect_cohort(&text);
                                set_cohort.set(detected);
                                if let Some(cohort) = detected.filter(|_| auto) {
                                    set_curriculum_id.set(curriculum_for_cohort(cohort).id.clone());
                                }
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

    let years = curricula()
        .iter()
        .map(|c| c.first_cohort.to_string())
        .collect::<Vec<_>>();
    let years_th = years.join(" และ ");
    let years_en = years.join(" and ");

    view! {
        <section class="max-w-[640px] mx-auto pt-6 sm:pt-10 text-center">
            <span class="inline-block text-xs font-medium rounded-full px-2.5 py-0.5 bg-accent-50 text-accent-700 mb-4">
                {move || if is_thai.get() { format!("หลักสูตร {years_th}") } else { format!("Curricula {years_en}") }}
            </span>
            <h1 class="text-[26px] sm:text-[30px] leading-snug font-semibold text-zinc-900 text-balance">
                {move || if is_thai.get() { "ตรวจว่าเรียนครบตามหลักสูตรหรือยัง" } else { "Check whether you've met your curriculum" }}
            </h1>
            <p class="mt-2 mb-7 text-[15px] text-zinc-600 text-balance">
                {move || if is_thai.get() {
                    "อัปโหลดใบแสดงผลการเรียนจาก SIS แล้วดูว่าแต่ละหมวดเหลืออีกกี่หน่วยกิต วิชาไหนยังไม่ได้เรียน และเทอมหน้าเปิดอะไรบ้าง"
                } else {
                    "Upload your SIS transcript to see how many credits each category still needs, which courses are left, and what's offered next term."
                }}
            </p>

            {move || if is_loading.get() {
                view! {
                    <div class="rounded-2xl border border-zinc-200 bg-white py-14 flex flex-col items-center gap-4" role="status" aria-live="polite">
                        <div class="relative">
                            <div class="w-11 h-11 rounded-full border-2 border-zinc-200"></div>
                            <div class="absolute inset-0 w-11 h-11 rounded-full border-2 border-accent-600 border-t-transparent animate-spin"></div>
                        </div>
                        <div>
                            <p class="text-sm font-medium text-zinc-800">{move || if is_thai.get() { "กำลังวิเคราะห์ใบแสดงผลการเรียน" } else { "Analysing your transcript" }}</p>
                            <p class="text-xs text-zinc-500 mt-0.5">{move || if is_thai.get() { "อ่านรายวิชาและตรวจกับหลักสูตร" } else { "Reading courses and checking them against the curriculum" }}</p>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div
                        class="group relative rounded-2xl border-[1.5px] border-dashed border-zinc-300 bg-white px-6 py-10 transition-colors hover:border-accent-500 hover:bg-accent-50/60 focus-within:border-accent-500 focus-within:ring-2 focus-within:ring-accent-500/30 cursor-pointer"
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
                        <div class="pointer-events-none flex flex-col items-center">
                            <div class="w-[52px] h-[52px] mb-3.5 rounded-[14px] grid place-items-center bg-zinc-50 border border-zinc-200 text-zinc-500">
                                <svg class="w-6 h-6" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M12 16V4m0 0-4 4m4-4 4 4"/><path d="M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"/></svg>
                            </div>
                            {move || if file_name.get().is_empty() {
                                view! {
                                    <p class="text-base font-semibold text-zinc-900">{move || if is_thai.get() { "วางไฟล์ PDF ที่นี่ หรือคลิกเพื่อเลือก" } else { "Drop a PDF here or click to choose" }}</p>
                                    <p class="text-[13.5px] text-zinc-500">{move || if is_thai.get() { "ใบแสดงผลการเรียนหรือผลการลงทะเบียน ภาษาไทยหรืออังกฤษ" } else { "Grade report or registration result, Thai or English" }}</p>
                                }.into_any()
                            } else {
                                view! {
                                    <p class="text-base font-semibold text-zinc-900 break-all">{file_name.get()}</p>
                                    <p class="text-[13.5px] text-zinc-500">{move || if is_thai.get() { "คลิกเพื่อเลือกไฟล์อื่น" } else { "Click to choose another file" }}</p>
                                }.into_any()
                            }}
                        </div>
                    </div>
                }.into_any()
            }}

            {move || error_msg.get().map(|err| view! {
                <div class="mt-4 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-left" role="alert">
                    <p class="text-sm font-semibold text-red-800">{move || if is_thai.get() { "วิเคราะห์ไม่สำเร็จ" } else { "Couldn't analyse the file" }}</p>
                    <p class="text-[13px] text-red-700 mt-0.5">{err}</p>
                </div>
            })}

            <div class="mt-5 flex flex-wrap justify-center gap-2.5">
                <select
                    class="h-10 max-w-full rounded-lg border border-zinc-200 bg-white px-3 text-sm text-zinc-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500"
                    aria-label={move || if is_thai.get() { "หลักสูตร" } else { "Curriculum" }}
                    prop:value=move || if auto_curriculum.get() { "auto".to_string() } else { curriculum_id.get() }
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        if v == "auto" {
                            set_auto_curriculum.set(true);
                        } else {
                            set_auto_curriculum.set(false);
                            set_curriculum_id.set(v);
                        }
                    }
                >
                    <option value="auto">{move || if is_thai.get() { "ตรวจหลักสูตรอัตโนมัติจากรหัสนักศึกษา" } else { "Detect curriculum from student ID" }}</option>
                    {curricula().iter().map(|c| view! { <option value=c.id.clone()>{c.name.clone()}</option> }).collect::<Vec<_>>()}
                </select>
                <button
                    class="h-10 inline-flex items-center gap-2 rounded-lg bg-zinc-900 px-4 text-sm font-medium text-white hover:bg-zinc-800 disabled:opacity-40 disabled:cursor-not-allowed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 focus-visible:ring-offset-2"
                    disabled={move || file_name.get().is_empty() || is_loading.get()}
                    on:click=on_start_analysis
                >
                    {move || if is_thai.get() { "วิเคราะห์" } else { "Analyse" }}
                    <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M5 12h14m-6-6 6 6-6 6"/></svg>
                </button>
            </div>

            <p class="mt-5 inline-flex items-center gap-2 text-[13px] text-zinc-500">
                <svg class="w-4 h-4 text-emerald-600 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" viewBox="0 0 24 24"><path d="M12 3 4 6v6c0 5 3.5 8 8 9 4.5-1 8-4 8-9V6l-8-3Z"/><path d="m9 12 2 2 4-4"/></svg>
                {move || if is_thai.get() { "ประมวลผลในเบราว์เซอร์ของคุณทั้งหมด ไฟล์ไม่ถูกส่งออกจากเครื่อง" } else { "Everything runs in your browser. The file never leaves your device." }}
            </p>

            <ol class="mt-8 grid gap-3 sm:grid-cols-3 text-left">
                <li class="rounded-xl border border-zinc-200 bg-white px-4 py-3.5">
                    <span class="text-xs text-zinc-400 tabular-nums">"01"</span>
                    <p class="font-semibold text-[14.5px] text-zinc-900">{move || if is_thai.get() { "ดาวน์โหลดจาก SIS" } else { "Download from SIS" }}</p>
                    <p class="text-[13px] text-zinc-500">{move || if is_thai.get() { "เมนูผลการเรียน แล้วพิมพ์เป็น PDF" } else { "Open your grades and print them to PDF" }}</p>
                </li>
                <li class="rounded-xl border border-zinc-200 bg-white px-4 py-3.5">
                    <span class="text-xs text-zinc-400 tabular-nums">"02"</span>
                    <p class="font-semibold text-[14.5px] text-zinc-900">{move || if is_thai.get() { "อัปโหลดที่นี่" } else { "Upload it here" }}</p>
                    <p class="text-[13px] text-zinc-500">{move || if is_thai.get() { "ระบบอ่านรหัสวิชา หน่วยกิต และเกรด" } else { "Codes, credits and grades are read for you" }}</p>
                </li>
                <li class="rounded-xl border border-zinc-200 bg-white px-4 py-3.5">
                    <span class="text-xs text-zinc-400 tabular-nums">"03"</span>
                    <p class="font-semibold text-[14.5px] text-zinc-900">{move || if is_thai.get() { "ดูสิ่งที่ยังขาด" } else { "See what's left" }}</p>
                    <p class="text-[13px] text-zinc-500">{move || if is_thai.get() { "พร้อมจำลองแผนเทอมถัดไป" } else { "And plan the next term" }}</p>
                </li>
            </ol>

            <div class="mt-6 flex justify-center">
                <SaveToggle/>
            </div>
        </section>
    }
}
