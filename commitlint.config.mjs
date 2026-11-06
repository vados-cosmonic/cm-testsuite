// Adapted from bytecodealliance/jco (Apache-2.0); see THIRD_PARTY.md.
export default {
    extends: ['@commitlint/config-conventional'],
    parserPreset: 'conventional-changelog-conventionalcommits',
    rules: {
        'type-enum': [2, 'always', [
            'build', 'chore', 'ci', 'debug', 'docs', 'feat', 'fix', 'perf',
            'refactor', 'release', 'revert', 'style', 'test', 'sec',
        ]],
        'scope-enum': [2, 'always', [
            'deps', 'deps-dev', 'examples', 'ci', 'ops', 'docs', 'tests', 'ai',
            'acquisition', 'catalog', 'delivery', 'exchange-plan', 'github',
            'results', 'wast-analysis', 'release', 'xtask', 'corpus', 'registry',
            'build', 'toolchains',
        ]],
        'scope-case': [2, 'always', 'lower-case'],
        'body-max-line-length': [1, 'always', 100],
        // Squash and merge-queue bodies after a Signed-off-by trailer parse as footer.
        'footer-max-line-length': [1, 'always', 100],
    },
};
