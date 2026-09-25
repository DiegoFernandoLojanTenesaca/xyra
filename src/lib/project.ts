import { openUrl } from '@tauri-apps/plugin-opener';

export const REPOSITORY = 'https://github.com/DiegoFernandoLojanTenesaca/xyra';

export const LINKS = {
  repository: REPOSITORY,
  reportIssue: `${REPOSITORY}/issues/new?template=bug.yml`,
  releases: `${REPOSITORY}/releases`,
};

/** Creator photos ship with the app, so Xyra never contacts GitHub to show them. */
export const CREATORS = [
  { name: 'Diego Fernando', user: 'DiegoFernandoLojanTenesaca', team: 'IndagaLab', role: 'creator', photo: '/creators/diego.jpg' },
  { name: '@jahirxtrap', user: 'jahirxtrap', team: 'Xynitra', role: 'coCreator', photo: '/creators/jahir.jpg' },
] as const;

export const githubProfile = (user: string) => `https://github.com/${user}`;

export const openExternal = (url: string) => void openUrl(url);
