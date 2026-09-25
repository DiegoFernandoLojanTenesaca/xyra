import type { AppError } from '../types';

export const toAppError = (error: unknown): AppError =>
  typeof error === 'object' && error !== null && 'code' in error ? (error as AppError) : { code: 'platform', detail: String(error) };
