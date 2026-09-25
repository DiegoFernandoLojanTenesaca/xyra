import type { AppError } from '../types';

/** Commands reject with an AppError; anything else comes from the bridge itself. */
export const toAppError = (error: unknown): AppError =>
  typeof error === 'object' && error !== null && 'code' in error ? (error as AppError) : { code: 'platform', detail: String(error) };
