import { FunctionComponent, h } from 'deps/preact';
import { useState, useEffect } from 'deps/preactHooks';

interface Props {
    start: number;
}

export const App: FunctionComponent<Props> = ({ start }) => {
    const [tick, setTick] = useState(0);
    useEffect(() => {
        const id = setInterval(() => {
            setTick((prev) => (prev + 1) % 2);
        });
        return () => clearInterval(id);
    }, []);

    return <div>{(start + tick) % 2 === 0 ? 'Hello' : 'Goodbye'}</div>;
};
