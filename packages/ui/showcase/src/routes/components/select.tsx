
import { createFileRoute } from '@tanstack/solid-router';
import { 
  Select, 
  SelectTrigger, 
  SelectContent, 
  SelectItem, 
  SelectValue 
} from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';
import { createSignal, For } from 'solid-js';

export const Route = createFileRoute('/components/select')({
  component: SelectPage,
});

function SelectPage() {
  const [selectedValue, setSelectedValue] = createSignal('');
  const [multiValue, setMultiValue] = createSignal<string[]>([]);

  const fruits = [
    { value: 'apple', label: 'Apple' },
    { value: 'banana', label: 'Banana' },
    { value: 'cherry', label: 'Cherry' },
    { value: 'date', label: 'Date' },
    { value: 'elderberry', label: 'Elderberry' }
  ];

  const countries = [
    { value: 'us', label: 'United States' },
    { value: 'ca', label: 'Canada' },
    { value: 'uk', label: 'United Kingdom' },
    { value: 'de', label: 'Germany' },
    { value: 'fr', label: 'France' }
  ];

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold mb-4" style={`color: rgb(var(--lightSlate-50))`}>Select</h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Dropdown selection component with single and multi-select capabilities.
        </p>
      </div>

      <ComponentDemo
        title="Basic Select"
        description="Simple single-value select dropdown"
      >
        <div class="space-y-4">
          <Select 
            options={fruits}
            value={selectedValue()} 
            onChange={setSelectedValue}
            itemComponent={props => <SelectItem item={props.item}>{props.item.rawValue.label}</SelectItem>}
          >
            <SelectTrigger>
              <SelectValue>{state => state.selectedOption()?.label || "Choose a fruit..."}</SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
          <div class="text-sm text-gray-400">
            Selected: {selectedValue() || 'None'}
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Multi-Select"
        description="Select multiple options from the dropdown"
      >
        <div class="space-y-4">
          <Select 
            multiple 
            options={fruits}
            value={multiValue()} 
            onChange={setMultiValue}
            itemComponent={props => <SelectItem item={props.item}>{props.item.rawValue.label}</SelectItem>}
          >
            <SelectTrigger>
              <SelectValue>{state => {
                const selected = state.selectedOptions();
                return selected.length > 0 
                  ? `${selected.length} selected` 
                  : "Choose multiple fruits...";
              }}</SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
          <div class="text-sm text-gray-400">
            Selected: {multiValue().length > 0 ? multiValue().map(val => fruits.find(f => f.value === val)?.label).join(', ') : 'None'}
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Different Options"
        description="Select with different option set"
      >
        <div class="space-y-4">
          <Select 
            options={countries}
            itemComponent={props => <SelectItem item={props.item}>{props.item.rawValue.label}</SelectItem>}
          >
            <SelectTrigger>
              <SelectValue>{state => state.selectedOption()?.label || "Choose a country..."}</SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Disabled Select"
        description="Select in disabled state"
      >
        <Select 
          disabled 
          options={fruits}
          itemComponent={props => <SelectItem item={props.item}>{props.item.rawValue.label}</SelectItem>}
        >
          <SelectTrigger>
            <SelectValue>{() => "This select is disabled"}</SelectValue>
          </SelectTrigger>
          <SelectContent />
        </Select>
      </ComponentDemo>

      <ComponentDemo
        title="Controlled Select with Labels"
        description="Select with controlled state and visual feedback"
      >
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">
              Favorite Fruit
            </label>
            <Select 
              options={fruits}
              value={selectedValue()} 
              onChange={setSelectedValue}
              itemComponent={props => <SelectItem item={props.item}>{props.item.rawValue.label}</SelectItem>}
            >
              <SelectTrigger>
                <SelectValue>{state => state.selectedOption()?.label || "Select your favorite..."}</SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </div>
          {selectedValue() && (
            <div class="p-3 bg-green-900/20 border border-green-700 rounded text-green-400 text-sm">
              Great choice! You selected: {fruits.find(f => f.value === selectedValue())?.label}
            </div>
          )}
        </div>
      </ComponentDemo>
    </div>
  );
}
