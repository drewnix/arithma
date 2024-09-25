import {StrictMode} from 'react'
import {createRoot} from 'react-dom/client'
// import App from './App.tsx'
import App from './App.tsx'
import './index.css'

createRoot(document.getElementById('root')!).render(
    <StrictMode>
        <App
         defaultLayout={[20, 32, 48]} navCollapsedSize={4}/>
    </StrictMode>,
)
