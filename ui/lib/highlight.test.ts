import { describe, it, expect } from 'vitest';
import { detectLang } from './highlight';

describe('detectLang', () => {
  it('maps TypeScript extensions', () => {
    expect(detectLang('src/app.ts')).toBe('typescript');
    expect(detectLang('Component.tsx')).toBe('typescript');
  });

  it('maps JavaScript extensions', () => {
    expect(detectLang('bundle.js')).toBe('javascript');
    expect(detectLang('page.jsx')).toBe('javascript');
  });

  it('maps Python, Rust, and shell extensions', () => {
    expect(detectLang('script.py')).toBe('python');
    expect(detectLang('lib.rs')).toBe('rust');
    expect(detectLang('run.sh')).toBe('bash');
  });

  it('maps web and config extensions', () => {
    expect(detectLang('index.html')).toBe('html');
    expect(detectLang('App.svelte')).toBe('svelte');
    expect(detectLang('data.json')).toBe('json');
    expect(detectLang('config.yaml')).toBe('yaml');
    expect(detectLang('Cargo.toml')).toBe('toml');
  });

  it('maps additional language extensions', () => {
    expect(detectLang('main.go')).toBe('go');
    expect(detectLang('App.java')).toBe('java');
    expect(detectLang('native.cpp')).toBe('cpp');
    expect(detectLang('Program.cs')).toBe('csharp');
    expect(detectLang('index.php')).toBe('php');
    expect(detectLang('Gemfile.rb')).toBe('ruby');
    expect(detectLang('query.sql')).toBe('sql');
  });

  it('returns empty string for unknown extensions', () => {
    expect(detectLang('readme.txt')).toBe('');
    expect(detectLang('noextension')).toBe('');
  });
});
