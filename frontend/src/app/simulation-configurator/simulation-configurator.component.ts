import {Component, Inject, OnInit} from '@angular/core';
import {MatDialogRef} from "@angular/material/dialog";
import {GraphServiceService} from "../service/graph-service.service";

@Component({
  selector: 'app-simulation-configurator',
  templateUrl: './simulation-configurator.component.html',
  styleUrls: ['./simulation-configurator.component.css']
})
export class SimulationConfiguratorComponent implements OnInit {

  //TODO: load Options from backend on Dialog Open
  graphOptions: string[] = ["someTestGraph.fmi", "another One"];
  strategyOptions: string[] = ["Greedy", "Something else"]

  constructor(
    public dialogRef: MatDialogRef<SimulationConfiguratorComponent>,
    private graphService: GraphServiceService
    ) {
  }

  ngOnInit(): void {
    //retrieve Dropdown Options here
    this.graphService.getGraphs().subscribe(
      data => {
        console.log(data);
        this.graphOptions = data;
      }
    )
  }

  cancel() {
    this.dialogRef.close();
  }

  confirm() {
    console.log("sending sim data to backend");
  }
}
