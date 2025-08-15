
import { createFileRoute } from '@tanstack/solid-router';
import { LoadingBar } from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';
import { createSignal, onMount } from 'solid-js';

export const Route = createFileRoute('/components/loadingbar')({
  component: LoadingbarPage,
});

function LoadingbarPage() {
  const [progress1, setProgress1] = createSignal(0);
  const [progress2, setProgress2] = createSignal(30);
  const [progress3, setProgress3] = createSignal(75);

  onMount(() => {
    // Animate first progress bar
    let value = 0;
    const interval = setInterval(() => {
      value += 1;
      setProgress1(value);
      if (value >= 100) {
        value = 0;
      }
    }, 100);

    return () => clearInterval(interval);
  });

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold mb-4" style={`color: rgb(var(--lightSlate-50))`}>LoadingBar</h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Progress indicator showing completion percentage of an operation.
        </p>
      </div>

      <ComponentDemo
        title="Animated Progress"
        description="Loading bar with animated progress"
      >
        <div class="space-y-4">
          <LoadingBar value={progress1()} max={100} />
          <div style={`color: rgb(var(--lightSlate-300))`} class="text-sm">
            Progress: {Math.round(progress1())}%
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Different Progress Values"
        description="Loading bars with various completion states"
      >
        <div class="space-y-4">
          <div>
            <LoadingBar value={progress2()} max={100} />
            <div style={`color: rgb(var(--lightSlate-300))`} class="text-sm mt-1">
              30% Complete
            </div>
          </div>
          <div>
            <LoadingBar value={progress3()} max={100} />
            <div style={`color: rgb(var(--lightSlate-300))`} class="text-sm mt-1">
              75% Complete
            </div>
          </div>
          <div>
            <LoadingBar value={100} max={100} />
            <div style={`color: rgb(var(--green-400))`} class="text-sm mt-1">
              100% Complete ✓
            </div>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Indeterminate Loading"
        description="Loading bar for unknown progress duration"
      >
        <LoadingBar indeterminate />
      </ComponentDemo>
    </div>
  );
}
