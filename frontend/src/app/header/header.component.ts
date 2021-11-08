import { Component, EventEmitter, OnInit, Output } from '@angular/core';
import { MatDialog } from '@angular/material/dialog';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { SimulationConfig } from '../data/SimulationConfig';

@Component({
  selector: 'app-header',
  templateUrl: './header.component.html',
  styleUrls: ['./header.component.css']
})
export class HeaderComponent implements OnInit {

  @Output() simConfigEvent = new EventEmitter<SimulationConfig>();

  constructor(
    private dialog: MatDialog
  ) { }

  ngOnInit(): void {
  }

  openSimulationConfigDialog() {

    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px'
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      this.simConfigEvent.emit(data);
    })
  }

}
