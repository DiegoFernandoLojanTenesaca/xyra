import { openUrl } from '@tauri-apps/plugin-opener';

export const REPOSITORY = 'https://github.com/DiegoFernandoLojanTenesaca/xyra';

export const LINKS = {
  repository: REPOSITORY,
  reportIssue: `${REPOSITORY}/issues/new?template=bug.yml`,
  releases: `${REPOSITORY}/releases`,
};

/** Creators, with their photos bundled in the app. */
export const CREATORS = [
  { user: 'DiegoFernandoLojanTenesaca', team: 'IndagaLab', role: 'creator', photo: '/creators/diego.jpg' },
  { user: 'jahirxtrap', team: 'Xynitra', role: 'coCreator', photo: '/creators/jahir.jpg' },
] as const;

export const githubProfile = (user: string) => `https://github.com/${user}`;

export const githubHandle = (user: string) => `@${user}`;

export const openExternal = (url: string) => void openUrl(url);
