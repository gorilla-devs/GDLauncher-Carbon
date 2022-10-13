import { describe, expect, test, assert } from "vitest";
import { fireEvent, render } from 'solid-testing-library';
import { Counter } from "../components/Counter";

describe("<Counter />", () => {
  test("Sum", () => {
    assert.equal(Math.sqrt(4), 2);
  });

  // test("renders", () => {
  //   const { container, unmount } = render(() => <Counter count={4} />);
  //   expect(container.innerHTML).toMatchSnapshot();
  //   unmount();
  // });

  // test('updates', async () => {
  //   const { container, unmount, queryByText } = render(() => (
  //     <Counter count={4} />
  //   ));
  //   const button: any = queryByText('x1');
  //   const buttonClicked = new Promise((resolve) => {
  //     const handler = (ev: any) => {
  //       button.removeEventListener('click', handler);
  //       resolve(ev);
  //     };
  //     button.addEventListener('click', handler);
  //   });
  //   fireEvent.click(button);
  //   await buttonClicked;
  //   expect(container.innerHTML).toMatchSnapshot();
  //   unmount();
  // });
});
