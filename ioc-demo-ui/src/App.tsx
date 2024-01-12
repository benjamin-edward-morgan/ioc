import InputTable from './components/InputTable';
import OutputTable from './components/OutputTable';
import ConnectionTable from './components/ConnectionTable';
import useIocWebsocketClient from './ioc/IocWebsocketClient';


import GithubLogo  from './assets/github-mark.svg';
import Chart from './components/Chart';


function App() {

  const websocketUrl = "ws://" + window.location.host + "/ws";
  // const websocketUrl = "ws://turdatron.local:8080/ws";
  // const websocketUrl = "ws://localhost:8080/ws";


  const [ioc, setter] = useIocWebsocketClient(websocketUrl);

  return (<>
    <header>
      <h1>IOC</h1>
    </header>
    <div className="content">



      <div className="sm-col" >
        <InputTable ioc={ioc} setter={setter} />
      </div>
      <div className="sm-col" >
        <OutputTable ioc={ioc} />
      </div>
      <div className="sm-col" >
        <ConnectionTable ioc={ioc} />
      </div>


      <div className="lg-col">
        <Chart ioc={ioc} />
      </div>
    </div>
    <footer>
      made with 😈 on planet earth

      <a href="http://github.com/benjamin-edward-morgan/" target="_blank" rel="noopener noreferrer">
        <span className="icon" style={{float: 'right'}}>
          <img src={GithubLogo} />
        </span>
      </a>
    </footer>
    </>);
}

export default App
