import {Component, Inject, OnInit} from '@angular/core';
import {MatDialogRef} from "@angular/material/dialog";

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
    ) { }

  ngOnInit(): void {
  }

  cancel() {
    this.dialogRef.close();
  }
}
