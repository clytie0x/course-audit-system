# Curriculum sources

The JSON files in this folder are transcribed from these official documents.
The PDFs are not stored in the repository. Each URL was fetched on
2026-11-22, and the SHA-256 identifies the exact bytes used. The sci.psu.ac.th
files answer 403 without a browser User-Agent and a Referer of the programme page.

Programme page: <https://www.sci.psu.ac.th/academic-programs/bachelors-degree/bachelor-of-science-computer-science/>

| Used for | Document | URL | Bytes | SHA-256 |
| --- | --- | --- | --- | --- |
| `cs2569.json` (structure, courses, electives, prerequisites) | มคอ.2 หลักสูตรปรับปรุง พ.ศ. 2569, 227 pages | <https://www.sci.psu.ac.th/wp-content/uploads/2026/06/2569-วท.บ.วิทยาการคอมพิวเตอร์.pdf> | 6044349 | `373afdba464b688e64f202a8d550a73ad468f42bf559c00ba72ca2866c41a3ba` |
| `cs2564.json` (structure, GPA rule, prerequisites) | มคอ.2 หลักสูตรปรับปรุง พ.ศ. 2564, 235 pages | <https://www.sci.psu.ac.th/wp-content/uploads/2023/10/ComputerScience25641.pdf> | 10839249 | `c25cd4016f1420be0dc122a070a198ac48271b7ba80b023a4387e307723b34ee` |
| `cs2564.json` (major structure, clusters) | โครงสร้างหลักสูตร 2564, 16 pages | <https://www.sci.psu.ac.th/wp-content/uploads/2021/09/โครงสร้าง-doc2-2-วิทยาการคอมพิวเตอร์.pdf> | 284661 | `fbe6d41b1ac996d8267fd8a10458d729bc996cf94eb5cfdb97975436bb4cca3c` |
| reference | แผนการศึกษา 2564, 4 pages | <https://www.sci.psu.ac.th/wp-content/uploads/2021/09/แผน-doc2-2-วิทยาการคอมพิวเตอร์.pdf> | 108003 | `7276d78977b22014697d000918437f3e238c4489e54b7ab17afbd7c92d81d4f2` |
| cross-check (cohort 2565 study manual) | คู่มือการศึกษา 2565 | <https://edumanual.psu.ac.th/file_pdf/2565_HDY_sci_13_1_1_1_55_627.pdf> | 641685 | `8159e74a784935b52d485014f21d0f9769d87e6104f25aef587a71caf9428083` |
| `cs2564.json` (revised core: 344-211/233/243) | คู่มือการศึกษา 2568 | <https://edumanual.psu.ac.th/file_pdf/2568_HDY_sci_3082.pdf> | 678859 | `5bd36560a5f59ef3b06c3045c85182bccfce903753aa7b0046e1886b65625c0c` |

Notes

- The 2564 GenEd strand lists come from a real cohort-2567 transcript, not from these
  documents. GE course catalogues by category live at <https://gened.psu.ac.th/>.
- Prerequisites were extracted from the "Prerequisite:" lines of each TQF2's course
  descriptions and kept only for courses listed in that curriculum. In both documents
  "รายวิชาบังคับเรียนก่อน" means the course must have been taken and graded before, with any grade.
