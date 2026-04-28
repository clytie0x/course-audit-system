import type { GenEdCurriculum, MajorCurriculum } from '$lib/audit/models';
import genEdJson from './gen_ed.json';
import majorJson from './major.json';

export const genEdCurriculum = genEdJson as GenEdCurriculum;
export const majorCurriculum = majorJson as MajorCurriculum;
