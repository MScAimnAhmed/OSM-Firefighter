import { Component, EventEmitter, OnInit, Output } from '@angular/core';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';

@Component({
  selector: 'app-turn-input',
  templateUrl: './turn-input.component.html',
  styleUrls: ['./turn-input.component.css']
})
export class TurnInputComponent implements OnInit {

  maxTurn = 0;
  currentTurn = 0;
  @Output('onChange') onChange = new EventEmitter<number>();
  currentTurnFormControl: FormControl;

  constructor() {
  }

  ngOnInit(): void {
    this.currentTurnFormControl = new FormControl(this.currentTurn, [Validators.required]);
    this.currentTurnFormControl.valueChanges.pipe(
      debounceTime(100),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentTurn);
    });
  }

  step(stepsize: number): void {
    if(stepsize > 0) {
      this.currentTurn = this.currentTurn + stepsize <= this.maxTurn ? this.currentTurn + stepsize : this.maxTurn;
    } else {
      this.currentTurn = this.currentTurn + stepsize >= 0 ? this.currentTurn + stepsize : 0;
    }
  }

}
