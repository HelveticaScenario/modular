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
        }, 1000);
        return () => clearInterval(id);
    }, []);
    return (
        <div class="p-6 max-w-sm mx-auto bg-white rounded-xl shadow-md flex items-center space-x-4">
            <div class="flex-shrink-0">
                <div class="pd ring-4">{(start + tick) % 2 === 0 ? 'Hello' : 'Goodbye'}</div>
            </div>
            <div>
                <div class="text-xl font-medium text-black">ChitChat</div>
                <p class="text-gray-500">You have a new message!</p>
            </div>
        </div>
    )
};
