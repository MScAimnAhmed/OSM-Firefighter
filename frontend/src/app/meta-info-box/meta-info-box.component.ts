import { Component, OnInit } from '@angular/core';
import { GraphServiceService } from '../service/graph-service.service';
import { StepMetaData } from '../data/StepMetaData';

@Component({
  selector: 'app-meta-info-box',
  templateUrl: './meta-info-box.component.html',
  styleUrls: ['./meta-info-box.component.css']
})
export class MetaInfoBoxComponent implements OnInit {

  turn: number;
  stepMeta: StepMetaData;

  loading: boolean;

  constructor(private graphservice: GraphServiceService,) { }

  ngOnInit(): void {
  }

  updateStepMetaData(turn: number) {
    this.graphservice.getStepMetaData(turn).subscribe(data => {
      console.log('done');
      this.turn = turn;
      this.stepMeta = data;
    })
  }
}
