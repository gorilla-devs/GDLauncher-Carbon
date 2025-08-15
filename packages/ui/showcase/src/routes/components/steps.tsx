
import { createFileRoute } from '@tanstack/solid-router';
import { Steps } from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';
import { createSignal } from 'solid-js';

export const Route = createFileRoute('/components/steps')({
  component: StepsPage,
});

function StepsPage() {
  const [currentStep, setCurrentStep] = createSignal(1);

  const steps = [
    { title: 'Account', description: 'Create your account' },
    { title: 'Profile', description: 'Setup your profile' },
    { title: 'Settings', description: 'Configure preferences' },
    { title: 'Complete', description: 'Finish setup' }
  ];

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold mb-4" style={`color: rgb(var(--lightSlate-50))`}>Steps</h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Step indicator component for multi-step processes and workflows.
        </p>
      </div>

      <ComponentDemo
        title="Basic Steps"
        description="Simple step indicator with progress"
      >
        <div class="space-y-6">
          <Steps steps={steps} currentStep={currentStep()} />
          <div class="flex space-x-2">
            <button
              onClick={() => setCurrentStep(Math.max(0, currentStep() - 1))}
              disabled={currentStep() === 0}
              class="px-4 py-2 rounded-md transition-colors disabled:opacity-50"
              style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
            >
              Previous
            </button>
            <button
              onClick={() => setCurrentStep(Math.min(steps.length - 1, currentStep() + 1))}
              disabled={currentStep() === steps.length - 1}
              class="px-4 py-2 rounded-md transition-colors disabled:opacity-50"
              style={`background-color: rgb(var(--primary-500)); color: white`}
            >
              Next
            </button>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Vertical Steps"
        description="Steps arranged vertically for narrow layouts"
      >
        <Steps 
          steps={steps} 
          currentStep={2} 
          orientation="vertical" 
        />
      </ComponentDemo>

      <ComponentDemo
        title="Steps with Icons"
        description="Steps can include custom icons for each step"
      >
        <Steps 
          steps={[
            { title: 'Start', icon: '🚀', description: 'Begin the process' },
            { title: 'Progress', icon: '⚙️', description: 'Work in progress' },
            { title: 'Finish', icon: '✅', description: 'All done!' }
          ]} 
          currentStep={1} 
        />
      </ComponentDemo>
    </div>
  );
}
