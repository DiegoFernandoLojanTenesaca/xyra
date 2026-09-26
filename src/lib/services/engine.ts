import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppEvent, Choices, Config, EngineState, PhoneDeviceView, PhoneLink, PhonePermissions } from '../types';

export const getState = () => invoke<EngineState>('get_state');
export const getConfig = () => invoke<Config>('get_config');
export const setConfig = (config: Config) => invoke<Config>('set_config', { config });
export const getChoices = () => invoke<Choices>('get_choices');
export const testOverlay = () => invoke<void>('test_overlay');
export const testVoice = () => invoke<void>('test_voice');
export const setBorderless = () => invoke<void>('set_borderless');
export const getPhoneLink = () => invoke<PhoneLink | null>('get_phone_link');
export const prepareWindowsForPhone = () => invoke<void>('prepare_windows_for_phone');
export const getPhoneDevices = () => invoke<PhoneDeviceView[]>('get_phone_devices');
export const setPhonePermissions = (id: string, permissions: PhonePermissions) => invoke<PhoneDeviceView[]>('set_phone_permissions', { id, permissions });
export const forgetPhone = (id: string) => invoke<PhoneDeviceView[]>('forget_phone', { id });

export const onEvent = <T>(event: AppEvent, handler: (payload: T) => void) => listen<T>(event, (e) => handler(e.payload));
