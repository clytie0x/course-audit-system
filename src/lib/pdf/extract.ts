/**
 * PDF text extraction via pdfjs-dist (npm, dynamic import).
 *
 * The whole module is lazily imported so the main bundle stays small —
 * pdfjs-dist + worker are only fetched when the user actually uploads a PDF.
 */

let pdfjsPromise: Promise<typeof import('pdfjs-dist')> | null = null;

async function getPdfjs(): Promise<typeof import('pdfjs-dist')> {
	if (!pdfjsPromise) {
		pdfjsPromise = (async () => {
			const pdfjs = await import('pdfjs-dist');
			const workerSrc = (await import('pdfjs-dist/build/pdf.worker.min.mjs?url')).default;
			pdfjs.GlobalWorkerOptions.workerSrc = workerSrc;
			return pdfjs;
		})();
	}
	return pdfjsPromise;
}

export async function extractTextFromPdf(file: File): Promise<string> {
	const pdfjs = await getPdfjs();
	const buffer = await file.arrayBuffer();
	const doc = await pdfjs.getDocument({ data: buffer }).promise;

	const pageTexts: string[] = [];
	for (let pageNum = 1; pageNum <= doc.numPages; pageNum++) {
		const page = await doc.getPage(pageNum);
		const content = await page.getTextContent();
		const items = content.items as Array<{ str?: string }>;
		const text = items.map((it) => it.str ?? '').join(' ');
		pageTexts.push(text);
	}

	await doc.destroy();
	return pageTexts.join('\n');
}
