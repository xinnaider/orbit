const ANDROID_CODENAMES = [
  'hammerhead',
  'shamu',
  'bullhead',
  'angler',
  'marlin',
  'sailfish',
  'walleye',
  'taimen',
  'blueline',
  'crosshatch',
  'flame',
  'coral',
  'sunfish',
  'redfin',
  'barbet',
  'oriole',
  'raven',
  'cheetah',
  'panther',
  'lynx',
  'felix',
  'akita',
  'caiman',
  'komodo',
  'tokay',
  'dolph',
  'husky',
  'shiba',
  'tangor',
  'comet',
];

export function generateSessionName(projectName: string): string {
  const codename = ANDROID_CODENAMES[Math.floor(Math.random() * ANDROID_CODENAMES.length)];
  return `${codename} · ${projectName}`;
}
